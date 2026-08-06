from __future__ import annotations

from pydantic import BaseModel, Field


class RoleGrant(BaseModel):
    principal: str
    role: str


class RolePermission(BaseModel):
    role: str
    resource: str
    action: str


class RbacPolicy(BaseModel):
    grants: list[RoleGrant] = Field(default_factory=list)
    permissions: list[RolePermission] = Field(default_factory=list)

    def roles_for(self, principal: str) -> set[str]:
        return {grant.role for grant in self.grants if grant.principal == principal}

    def allows(self, principal: str, resource: str, action: str) -> bool:
        roles = self.roles_for(principal)
        return any(
            permission.role in roles
            and permission.resource == resource
            and permission.action == action
            for permission in self.permissions
        )
