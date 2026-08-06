from __future__ import annotations

from pydantic import BaseModel

from querygraph.odrl import Action, Policy
from querygraph.rbac import RbacPolicy
from querygraph.typedid import AccessReceipt


class OdrlDecision(BaseModel):
    principal: str
    resource: str
    action: str
    rbac_allowed: bool
    odrl_allowed: bool
    allowed: bool
    receipt: AccessReceipt


class OdrlRightsLayer(BaseModel):
    """ODRL policy evaluation with RBAC and QueryGraph access receipts."""

    rbac: RbacPolicy
    odrl: Policy

    def decide(self, principal: str, resource: str, action: Action) -> OdrlDecision:
        rbac_allowed = self.rbac.allows(principal, resource, action.value)
        odrl_allowed = self.odrl.allows(principal, action)
        allowed = rbac_allowed and odrl_allowed
        receipt = AccessReceipt(
            principal=principal,
            resource=resource,
            action=action.iri(),
            allowed=allowed,
            reason=(
                "RBAC and ODRL permitted action"
                if allowed
                else "RBAC or ODRL denied action"
            ),
            policy_id=self.odrl.id,
        )
        return OdrlDecision(
            principal=principal,
            resource=resource,
            action=action.iri(),
            rbac_allowed=rbac_allowed,
            odrl_allowed=odrl_allowed,
            allowed=allowed,
            receipt=receipt,
        )
