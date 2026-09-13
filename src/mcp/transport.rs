//! Bounded, session-owned governed work while the input thread reads controls.
use super::*;
use std::io::{BufRead, Read, Write};
use std::sync::{Arc, Mutex};
use tokio::task::{AbortHandle, JoinSet};

const MAX_IN_FLIGHT: usize = 16;

impl McpServer {
    pub(crate) fn serve_io<W: Write + Send + 'static>(
        &mut self,
        input: &mut impl BufRead,
        output: Arc<Mutex<W>>,
    ) -> Result<()> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()?;
        let mut tasks = JoinSet::<(String, Result<()>)>::new();
        let mut requests = BTreeMap::<String, AbortHandle>::new();
        let mut result = (|| {
            loop {
                while let Some(completed) = tasks.try_join_next_with_id() {
                    match completed {
                        Ok((task_id, (key, result))) => {
                            if requests
                                .get(&key)
                                .is_some_and(|handle| handle.id() == task_id)
                            {
                                requests.remove(&key);
                            }
                            result?;
                        }
                        Err(error) if error.is_cancelled() => {}
                        Err(error) => return Err(error.into()),
                    }
                }
                let mut bytes = Vec::new();
                let count = input
                    .by_ref()
                    .take((MAX_MCP_MESSAGE_BYTES + 1) as u64)
                    .read_until(b'\n', &mut bytes)?;
                if count == 0 {
                    return Ok(());
                }
                if count > MAX_MCP_MESSAGE_BYTES {
                    write_response(
                        &output,
                        &rpc_failure(Value::Null, -32600, "message exceeds size limit"),
                    )?;
                    anyhow::bail!("MCP input exceeded the message limit; session closed");
                }
                let line = std::str::from_utf8(&bytes)?;
                if line.trim().is_empty() {
                    continue;
                }
                // A worker can finish while the input thread waits for a line.
                // Reap before testing ID reuse or the concurrency bound.
                while let Some(completed) = tasks.try_join_next_with_id() {
                    match completed {
                        Ok((task_id, (key, result))) => {
                            if requests
                                .get(&key)
                                .is_some_and(|handle| handle.id() == task_id)
                            {
                                requests.remove(&key);
                            }
                            result?;
                        }
                        Err(error) if error.is_cancelled() => {}
                        Err(error) => return Err(error.into()),
                    }
                }
                match self.parse_line(line) {
                    Incoming::Modern(request) => {
                        let key = request.id.to_string();
                        if requests.contains_key(&key) || tasks.len() >= MAX_IN_FLIGHT {
                            write_response(
                                &output,
                                &modern::Error::new(
                                    -32600,
                                    "request ID already in flight or capacity exceeded",
                                )
                                .response(request.id)
                                .to_string(),
                            )?;
                            continue;
                        }
                        if !request.governed() {
                            write_response(
                                &output,
                                &modern::complete(
                                    request.id.clone(),
                                    self.modern.dispatch_sync(&request),
                                )
                                .to_string(),
                            )?;
                            continue;
                        }
                        let worker = self.modern.clone();
                        let output = output.clone();
                        let task_key = key.clone();
                        let handle = tasks.spawn_on(
                            async move {
                                let result = match tokio::time::timeout(
                                    std::time::Duration::from_secs(30),
                                    worker.dispatch(&request),
                                )
                                .await
                                {
                                    Ok(result) => result,
                                    Err(_) => Err(modern::Error::new(1001, "request timed out")),
                                };
                                (
                                    task_key,
                                    write_response(
                                        &output,
                                        &modern::complete(request.id, result).to_string(),
                                    ),
                                )
                            },
                            runtime.handle(),
                        );
                        requests.insert(key, handle);
                    }
                    Incoming::Notification => {}
                    Incoming::Cancelled(id) => {
                        if let Some(handle) = requests.remove(&id.to_string()) {
                            handle.abort();
                        }
                    }
                    Incoming::Reply(reply) => write_response(&output, &reply)?,
                    Incoming::Request { id, method, params } => {
                        let key = id.to_string();
                        if requests.contains_key(&key) {
                            write_response(
                                &output,
                                &rpc_failure(id, -32600, "request ID is already in flight"),
                            )?;
                            continue;
                        }
                        let governed = method == "tools/call"
                            && matches!(
                                params["name"].as_str(),
                                Some(
                                    "plan_pinax_scan"
                                        | "execute_pinax_scan"
                                        | "discover_pinax_tables"
                                        | "discover_pinax_ontology"
                                )
                            )
                            && matches!(self.session, SessionState::Ready(_));
                        if governed {
                            if tasks.len() >= MAX_IN_FLIGHT {
                                write_response(
                                    &output,
                                    &rpc_failure(
                                        id,
                                        -32000,
                                        "too many in-flight governed requests",
                                    ),
                                )?;
                                continue;
                            }
                            let worker = McpServer {
                                registry: BTreeMap::new(),
                                session: self.session,
                                registry_service: self.registry_service.clone(),
                                modern: self.modern.clone(),
                            };
                            let output = output.clone();
                            let task_key = key.clone();
                            let handle = tasks.spawn_on(
                                async move {
                                    let result = match tokio::time::timeout(
                                        std::time::Duration::from_secs(30),
                                        worker.call_registry_tool(&params),
                                    ).await {
                                        Ok(result) => result,
                                        Err(_) => Ok(json!({"content":[{"type":"text","text":"governed request timed out"}],"isError":true})),
                                    };
                                    (
                                        task_key,
                                        write_response(&output, &encode_response(id, result)),
                                    )
                                },
                                runtime.handle(),
                            );
                            requests.insert(key, handle);
                        } else {
                            write_response(
                                &output,
                                &encode_response(id, self.dispatch(&method, &params)),
                            )?;
                        }
                    }
                }
            }
        })();
        // EOF/errors own cancellation and joining; no governed task outlives this session.
        tasks.abort_all();
        runtime.block_on(async {
            while let Some(completed) = tasks.join_next().await {
                let completion = match completed {
                    Ok((_, completion)) => completion,
                    Err(error) if error.is_cancelled() => Ok(()),
                    Err(error) => Err(error.into()),
                };
                if result.is_ok() {
                    result = completion;
                }
            }
        });
        result
    }
}

fn write_response<W: Write>(output: &Mutex<W>, response: &str) -> Result<()> {
    let mut output = output
        .lock()
        .map_err(|_| anyhow::anyhow!("MCP output lock poisoned"))?;
    output.write_all(response.as_bytes())?;
    output.write_all(b"\n")?;
    output.flush()?;
    Ok(())
}
