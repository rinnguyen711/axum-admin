use crate::{adapter::DataAdapter, error::AdminError, field::Field};
use serde_json::Value;
use std::{collections::HashMap, future::Future, marker::PhantomData, pin::Pin};

pub enum ActionTarget {
    List,
    Detail,
}

pub struct ActionContext {
    pub ids: Vec<Value>,
    pub params: HashMap<String, String>,
}

pub enum ActionResult {
    Success(String),
    Redirect(String),
    Error(String),
}

type ActionHandler = Box<
    dyn Fn(ActionContext) -> Pin<Box<dyn Future<Output = Result<ActionResult, AdminError>> + Send>>
        + Send
        + Sync,
>;

type BeforeSaveHook =
    Box<dyn Fn(&mut HashMap<String, Value>) -> Result<(), AdminError> + Send + Sync>;

type AfterDeleteHook = Box<dyn Fn(&Value) -> Result<(), AdminError> + Send + Sync>;

pub struct CustomAction {
    pub name: String,
    pub label: String,
    pub target: ActionTarget,
    pub confirm: Option<String>,
    pub handler: ActionHandler,
}

impl CustomAction {
    pub fn builder(name: &str, label: &str) -> CustomActionBuilder {
        CustomActionBuilder {
            name: name.to_string(),
            label: label.to_string(),
            target: ActionTarget::List,
            confirm: None,
        }
    }
}

pub struct CustomActionBuilder {
    name: String,
    label: String,
    target: ActionTarget,
    confirm: Option<String>,
}

impl CustomActionBuilder {
    pub fn target(mut self, target: ActionTarget) -> Self {
        self.target = target;
        self
    }

    pub fn confirm(mut self, message: &str) -> Self {
        self.confirm = Some(message.to_string());
        self
    }

    pub fn handler<F, Fut>(self, f: F) -> CustomAction
    where
        F: Fn(ActionContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<ActionResult, AdminError>> + Send + 'static,
    {
        CustomAction {
            name: self.name,
            label: self.label,
            target: self.target,
            confirm: self.confirm,
            handler: Box::new(move |ctx| Box::pin(f(ctx))),
        }
    }
}

pub struct EntityAdmin {
    pub entity_name: String,
    pub label: String,
    pub fields: Vec<Field>,
    pub list_display: Vec<String>,
    pub search_fields: Vec<String>,
    pub filter_fields: Vec<String>,
    pub filters: Vec<Field>,
    pub actions: Vec<CustomAction>,
    pub adapter: Option<Box<dyn DataAdapter>>,
    pub before_save: Option<BeforeSaveHook>,
    pub after_delete: Option<AfterDeleteHook>,
    _marker: PhantomData<()>,
}

impl EntityAdmin {
    pub fn new<T>(_entity: &str) -> Self {
        Self {
            entity_name: _entity.to_string(),
            label: crate::field::default_label(_entity),
            fields: Vec::new(),
            list_display: Vec::new(),
            search_fields: Vec::new(),
            filter_fields: Vec::new(),
            filters: Vec::new(),
            actions: Vec::new(),
            adapter: None,
            before_save: None,
            after_delete: None,
            _marker: PhantomData,
        }
    }

    #[cfg(feature = "seaorm")]
    pub fn from_entity<E>(name: &str) -> Self
    where
        E: sea_orm::EntityTrait,
        E::Column: sea_orm::ColumnTrait,
    {
        let fields = crate::adapters::seaorm::seaorm_fields_for::<E>();
        Self {
            entity_name: name.to_string(),
            label: crate::field::default_label(name),
            fields,
            list_display: Vec::new(),
            search_fields: Vec::new(),
            filter_fields: Vec::new(),
            filters: Vec::new(),
            actions: Vec::new(),
            adapter: None,
            before_save: None,
            after_delete: None,
            _marker: PhantomData,
        }
    }

    pub fn label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

    pub fn field(mut self, field: Field) -> Self {
        if let Some(pos) = self.fields.iter().position(|f| f.name == field.name) {
            self.fields[pos] = field;
        } else {
            self.fields.push(field);
        }
        self
    }

    pub fn list_display(mut self, fields: Vec<String>) -> Self {
        self.list_display = fields;
        self
    }

    pub fn search_fields(mut self, fields: Vec<String>) -> Self {
        self.search_fields = fields;
        self
    }

    pub fn filter_fields(mut self, fields: Vec<&str>) -> Self {
        self.filter_fields = fields.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn filter(mut self, field: Field) -> Self {
        if let Some(pos) = self.filters.iter().position(|f| f.name == field.name) {
            self.filters[pos] = field;
        } else {
            self.filters.push(field);
        }
        self
    }

    pub fn adapter(mut self, adapter: Box<dyn DataAdapter>) -> Self {
        self.adapter = Some(adapter);
        self
    }

    pub fn action(mut self, action: CustomAction) -> Self {
        self.actions.push(action);
        self
    }

    pub fn before_save<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut HashMap<String, Value>) -> Result<(), AdminError> + Send + Sync + 'static,
    {
        self.before_save = Some(Box::new(f));
        self
    }

    pub fn after_delete<F>(mut self, f: F) -> Self
    where
        F: Fn(&Value) -> Result<(), AdminError> + Send + Sync + 'static,
    {
        self.after_delete = Some(Box::new(f));
        self
    }
}
