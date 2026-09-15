use crate::{
    ActionFunction, ComponentFunction, CriticalOperation, DomainType, EnumDef, FormSchema,
    InherentMethod, JsonSchema, LayoutFunction, Model, PageFunction, Permission, Program,
    PureFunction, QueryFunction,
};

impl Program {
    pub fn domain_type(&self, name: &str) -> Option<&DomainType> {
        self.domain_types.iter().find(|value| value.name == name)
    }
    pub fn domain_type_by_name(&self, name: &str) -> Option<(u16, &DomainType)> {
        self.domain_types
            .iter()
            .enumerate()
            .find(|(_, value)| value.name == name)
            .and_then(|(index, value)| u16::try_from(index).ok().map(|id| (id, value)))
    }
    pub fn domain_type_by_id(&self, id: u16) -> Option<&DomainType> {
        self.domain_types.get(id as usize)
    }
    pub fn representation_type(&self, ty: crate::ValueType) -> Option<crate::ValueType> {
        match ty {
            crate::ValueType::Domain(id) if id == crate::SAFE_HTML_DOMAIN_ID => {
                Some(crate::ValueType::String)
            }
            crate::ValueType::Domain(id) => self.domain_type_by_id(id).map(|domain| domain.base),
            crate::ValueType::Credential(_) => Some(crate::ValueType::String),
            other => Some(other),
        }
    }
    pub fn enum_by_name(&self, name: &str) -> Option<(u16, &EnumDef)> {
        self.enums
            .iter()
            .enumerate()
            .find(|(_, v)| v.name == name)
            .and_then(|(i, v)| u16::try_from(i).ok().map(|id| (id, v)))
    }
    pub fn enum_by_id(&self, id: u16) -> Option<&EnumDef> {
        self.enums.get(id as usize)
    }
    pub fn permission(&self, name: &str) -> Option<&Permission> {
        self.permissions.iter().find(|v| v.name == name)
    }
    pub fn critical_operation(&self, name: &str) -> Option<&CriticalOperation> {
        self.critical_operations.iter().find(|v| v.name == name)
    }
    pub fn model(&self, name: &str) -> Option<&Model> {
        self.models.iter().find(|v| v.name == name)
    }
    pub fn form(&self, name: &str) -> Option<&FormSchema> {
        self.forms.iter().find(|v| v.name == name)
    }
    pub fn json_schema(&self, name: &str) -> Option<&JsonSchema> {
        self.json_schemas.iter().find(|v| v.name == name)
    }
    pub fn json_schema_by_name(&self, name: &str) -> Option<(u16, &JsonSchema)> {
        self.json_schemas
            .iter()
            .enumerate()
            .find(|(_, v)| v.name == name)
            .and_then(|(index, schema)| u16::try_from(index).ok().map(|id| (id, schema)))
    }
    pub fn json_schema_by_id(&self, id: u16) -> Option<&JsonSchema> {
        self.json_schemas.get(id as usize)
    }
    pub fn component(&self, name: &str) -> Option<&ComponentFunction> {
        self.components.iter().find(|v| v.name == name)
    }
    pub fn layout(&self, name: &str) -> Option<&LayoutFunction> {
        self.layouts.iter().find(|v| v.name == name)
    }
    pub fn query(&self, name: &str) -> Option<&QueryFunction> {
        self.queries.iter().find(|v| v.name == name)
    }
    pub fn pure_function(&self, name: &str) -> Option<&PureFunction> {
        self.pure_functions.iter().find(|v| v.name == name)
    }
    pub fn inherent_method(&self, target: &str, name: &str) -> Option<&InherentMethod> {
        self.inherent_methods
            .iter()
            .find(|method| method.target == target && method.name == name)
    }
    pub fn page(&self, name: &str) -> Option<&PageFunction> {
        self.pages.iter().find(|v| v.name == name)
    }
    pub fn action(&self, name: &str) -> Option<&ActionFunction> {
        self.actions.iter().find(|v| v.name == name)
    }
}
