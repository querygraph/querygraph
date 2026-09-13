use super::*;

#[test]
fn builds_all_four_semantic_layers() {
    let output = AiNavigator.build(NavigatorInput {
        dataset_name: "Hazard vocabulary".to_string(),
        description: "Controlled vocabulary with multilingual technical terms".to_string(),
        landing_page: "https://querygraph.ai/datasets/hazards".to_string(),
        data_url: "https://querygraph.ai/datasets/hazards.csv".to_string(),
        creator: "QueryGraph".to_string(),
        agent_name: "AI Navigator".to_string(),
    });

    assert_eq!(output.croissant["@type"], "sc:Dataset");
    assert_eq!(output.cdif["@type"], "dcat:Dataset");
    assert!(output.did.id.starts_with("did:oyd:zQm"));
    assert_eq!(output.odrl["@type"], "odrl:Policy");
}
