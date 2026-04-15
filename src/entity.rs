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
    pub icon: Option<String>,
    pub class: Option<String>,
    pub handler: ActionHandler,
}

impl CustomAction {
    pub fn builder(name: &str, label: &str) -> CustomActionBuilder {
        CustomActionBuilder {
            name: name.to_string(),
            label: label.to_string(),
            target: ActionTarget::List,
            confirm: None,
            icon: None,
            class: None,
        }
    }
}

pub struct CustomActionBuilder {
    name: String,
    label: String,
    target: ActionTarget,
    confirm: Option<String>,
    icon: Option<String>,
    class: Option<String>,
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

    pub fn icon(mut self, icon_class: &str) -> Self {
        self.icon = Some(icon_class.to_string());
        self
    }

    pub fn class(mut self, css_class: &str) -> Self {
        self.class = Some(css_class.to_string());
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
            icon: self.icon,
            class: self.class,
            handler: Box::new(move |ctx| Box::pin(f(ctx))),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct EntityPermissions {
    /// Required permission to list records and open the edit form.
    pub view: Option<String>,
    /// Required permission to create a new record.
    pub create: Option<String>,
    /// Required permission to submit an edit.
    pub edit: Option<String>,
    /// Required permission to delete a record.
    pub delete: Option<String>,
}

pub struct EntityAdmin {
    pub entity_name: String,
    pub label: String,
    pub icon: String,
    pub group: Option<String>,
    pub pk_field: String,
    pub fields: Vec<Field>,
    pub list_display: Vec<String>,
    pub search_fields: Vec<String>,
    pub filter_fields: Vec<String>,
    pub filters: Vec<Field>,
    pub actions: Vec<CustomAction>,
    pub bulk_delete: bool,
    pub bulk_export: bool,
    pub adapter: Option<Box<dyn DataAdapter>>,
    pub before_save: Option<BeforeSaveHook>,
    pub after_delete: Option<AfterDeleteHook>,
    pub permissions: EntityPermissions,
    _marker: PhantomData<()>,
}

impl EntityAdmin {
    pub fn new<T>(_entity: &str) -> Self {
        Self {
            entity_name: _entity.to_string(),
            label: crate::field::default_label(_entity),
            icon: "fa-solid fa-layer-group".to_string(),
            group: None,
            pk_field: "id".to_string(),
            fields: Vec::new(),
            list_display: Vec::new(),
            search_fields: Vec::new(),
            filter_fields: Vec::new(),
            filters: Vec::new(),
            actions: Vec::new(),
            bulk_delete: true,
            bulk_export: true,
            adapter: None,
            before_save: None,
            after_delete: None,
            permissions: EntityPermissions::default(),
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
            icon: "fa-solid fa-layer-group".to_string(),
            group: None,
            pk_field: "id".to_string(),
            fields,
            list_display: Vec::new(),
            search_fields: Vec::new(),
            filter_fields: Vec::new(),
            filters: Vec::new(),
            actions: Vec::new(),
            bulk_delete: true,
            bulk_export: true,
            adapter: None,
            before_save: None,
            after_delete: None,
            permissions: EntityPermissions::default(),
            _marker: PhantomData,
        }
    }

    pub fn label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

    pub fn pk_field(mut self, pk: &str) -> Self {
        self.pk_field = pk.to_string();
        self
    }

    /// Set the Font Awesome icon class for this entity in the sidebar and dashboard.
    /// Defaults to `"fa-solid fa-layer-group"`.
    pub fn icon(mut self, icon: &str) -> Self {
        self.icon = icon.to_string();
        self
    }

    /// Assign this entity to a named sidebar group. Entities sharing the same
    /// group label are collapsed under a single expandable section.
    pub fn group(mut self, group: &str) -> Self {
        self.group = Some(group.to_string());
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

    pub fn filter_fields(mut self, fields: Vec<String>) -> Self {
        self.filter_fields = fields;
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

    pub fn bulk_delete(mut self, enabled: bool) -> Self {
        self.bulk_delete = enabled;
        self
    }

    pub fn bulk_export(mut self, enabled: bool) -> Self {
        self.bulk_export = enabled;
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

    /// Require `perm` to list or view this entity.
    pub fn require_view(mut self, perm: &str) -> Self {
        self.permissions.view = Some(perm.to_string());
        self
    }

    /// Require `perm` to create a new record.
    pub fn require_create(mut self, perm: &str) -> Self {
        self.permissions.create = Some(perm.to_string());
        self
    }

    /// Require `perm` to edit an existing record.
    pub fn require_edit(mut self, perm: &str) -> Self {
        self.permissions.edit = Some(perm.to_string());
        self
    }

    /// Require `perm` to delete a record.
    pub fn require_delete(mut self, perm: &str) -> Self {
        self.permissions.delete = Some(perm.to_string());
        self
    }

    /// Shortcut: require `role` for all four actions (view, create, edit, delete).
    pub fn require_role(mut self, role: &str) -> Self {
        let s = role.to_string();
        self.permissions.view = Some(s.clone());
        self.permissions.create = Some(s.clone());
        self.permissions.edit = Some(s.clone());
        self.permissions.delete = Some(s);
        self
    }
}

/// A named group of entities that renders as a collapsible section in the sidebar.
/// Register it with `AdminApp::register()` the same way as a plain `EntityAdmin`.
pub struct EntityGroupAdmin {
    pub label: String,
    pub icon: Option<String>,
    entities: Vec<EntityAdmin>,
}

impl EntityGroupAdmin {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            icon: None,
            entities: Vec::new(),
        }
    }

    /// Optional Font Awesome icon shown next to the group label in the sidebar.
    pub fn icon(mut self, icon: &str) -> Self {
        self.icon = Some(icon.to_string());
        self
    }

    /// Add an entity to this group.
    pub fn register(mut self, entity: EntityAdmin) -> Self {
        self.entities.push(entity);
        self
    }

    /// Consume the group and return its entities with the group label stamped on each.
    pub(crate) fn into_entities(self) -> Vec<EntityAdmin> {
        self.entities
            .into_iter()
            .map(|mut e| {
                e.group = Some(self.label.clone());
                e
            })
            .collect()
    }
}
