use crate::{
    ActionFunction, ComponentFunction, CriticalOperation, DomainType, EnumDef, FormSchema,
    InherentMethod, Integration, JsonSchema, LayoutFunction, Model, ModuleVisibility, PageFunction,
    Permission, ProductionPolicy, PureFunction, QueryFunction, ResourceUse, Route, SecurityEvent,
    SymbolVisibility, Webhook,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Program {
    pub domain_types: Vec<DomainType>,
    pub enums: Vec<EnumDef>,
    pub models: Vec<Model>,
    pub permissions: Vec<Permission>,
    pub critical_operations: Vec<CriticalOperation>,
    pub security_events: Vec<SecurityEvent>,
    pub webhooks: Vec<Webhook>,
    pub integrations: Vec<Integration>,
    pub production: Option<ProductionPolicy>,
    pub queries: Vec<QueryFunction>,
    pub pure_functions: Vec<PureFunction>,
    pub inherent_methods: Vec<InherentMethod>,
    pub pages: Vec<PageFunction>,
    pub actions: Vec<ActionFunction>,
    pub routes: Vec<Route>,
    pub forms: Vec<FormSchema>,
    pub json_schemas: Vec<JsonSchema>,
    pub components: Vec<ComponentFunction>,
    pub layouts: Vec<LayoutFunction>,
    pub resource_uses: Vec<ResourceUse>,
    pub visibilities: Vec<SymbolVisibility>,
    pub module_visibilities: Vec<ModuleVisibility>,
}
