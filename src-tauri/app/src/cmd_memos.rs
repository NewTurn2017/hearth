use crate::AppState;
use hearth_core::audit::Source;
use hearth_core::memos::{self, NewMemo, UpdateMemo};
use hearth_core::models::{Memo, MemoTag};
use serde::{Deserialize, Deserializer};
use tauri::State;

#[derive(Debug, Clone, PartialEq)]
pub enum PatchField<T> {
    Unset,
    Null,
    Value(T),
}

impl<T> Default for PatchField<T> {
    fn default() -> Self {
        Self::Unset
    }
}

impl<T> PatchField<T> {
    fn into_nullable_patch(self) -> Option<Option<T>> {
        match self {
            Self::Unset => None,
            Self::Null => Some(None),
            Self::Value(value) => Some(Some(value)),
        }
    }
}

impl<'de, T> Deserialize<'de> for PatchField<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<T>::deserialize(deserializer).map(|value| match value {
            Some(value) => Self::Value(value),
            None => Self::Null,
        })
    }
}

#[tauri::command]
pub fn get_memos(state: State<'_, AppState>) -> Result<Vec<Memo>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    memos::list(&db).map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
pub struct MemoInput {
    pub content: String,
    pub color: Option<String>,
    pub project_id: Option<i64>,
    pub font_size: Option<String>,
    pub is_bold: Option<bool>,
    pub focus_x: Option<f64>,
    pub focus_y: Option<f64>,
    pub tag_names: Option<Vec<String>>,
}

#[tauri::command]
pub fn create_memo(state: State<'_, AppState>, data: MemoInput) -> Result<Memo, String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    let color = data.color.as_deref().unwrap_or("yellow");
    memos::create(
        &mut db,
        Source::App,
        &NewMemo {
            content: &data.content,
            color,
            project_id: data.project_id,
            font_size: data.font_size.as_deref(),
            is_bold: data.is_bold,
            focus_x: data.focus_x,
            focus_y: data.focus_y,
            tag_names: data.tag_names.unwrap_or_default(),
        },
    )
    .map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
pub struct UpdateMemoInput {
    pub content: Option<String>,
    pub color: Option<String>,
    /// Omit leaves unchanged, `null` detaches, value attaches.
    #[serde(default)]
    pub project_id: PatchField<i64>,
    pub font_size: Option<String>,
    pub is_bold: Option<bool>,
    #[serde(default)]
    pub focus_x: PatchField<f64>,
    #[serde(default)]
    pub focus_y: PatchField<f64>,
    pub tag_names: Option<Vec<String>>,
}

#[tauri::command]
pub fn update_memo(
    state: State<'_, AppState>,
    id: i64,
    fields: UpdateMemoInput,
) -> Result<Memo, String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    memos::update(
        &mut db,
        Source::App,
        id,
        &UpdateMemo {
            content: fields.content.as_deref(),
            color: fields.color.as_deref(),
            project_id: fields.project_id.into_nullable_patch(),
            font_size: fields.font_size.as_deref(),
            is_bold: fields.is_bold,
            focus_x: fields.focus_x.into_nullable_patch(),
            focus_y: fields.focus_y.into_nullable_patch(),
            tag_names: fields.tag_names,
        },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_memo(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    memos::delete(&mut db, Source::App, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reorder_memos(state: State<'_, AppState>, ids: Vec<i64>) -> Result<(), String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    memos::reorder(&mut db, &ids).map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
pub struct UpdateMemoByNumberInput {
    pub content: Option<String>,
}

#[tauri::command]
pub fn update_memo_by_number(
    state: State<'_, AppState>,
    number: i64,
    fields: UpdateMemoByNumberInput,
) -> Result<Memo, String> {
    if number < 1 {
        return Err(format!("#{} 메모를 찾을 수 없음", number));
    }
    let new_content = fields
        .content
        .ok_or_else(|| "content is required for update_by_number".to_string())?;
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    memos::update_by_number(&mut db, Source::App, number, &new_content)
        .map_err(|_| format!("#{} 메모를 찾을 수 없음", number))
}

#[derive(Debug, Deserialize)]
pub struct MemoTagInput {
    pub name: String,
    pub color: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMemoTagInput {
    pub name: Option<String>,
    pub color: Option<String>,
    pub sort_order: Option<i64>,
}

#[tauri::command]
pub fn get_memo_tags(state: State<'_, AppState>) -> Result<Vec<MemoTag>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    memos::list_memo_tags(&db).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_memo_tag(state: State<'_, AppState>, input: MemoTagInput) -> Result<MemoTag, String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    memos::create_memo_tag(&mut db, Source::App, &input.name, input.color.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_memo_tag(
    state: State<'_, AppState>,
    id: i64,
    fields: UpdateMemoTagInput,
) -> Result<MemoTag, String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    memos::update_memo_tag(
        &mut db,
        Source::App,
        id,
        &memos::UpdateMemoTag {
            name: fields.name.as_deref(),
            color: fields.color.as_deref(),
            sort_order: fields.sort_order,
        },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_memo_tag(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    memos::delete_memo_tag(&mut db, Source::App, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reorder_memo_tags(state: State<'_, AppState>, ids: Vec<i64>) -> Result<(), String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    memos::reorder_memo_tags(&mut db, &ids).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_memo_by_number(state: State<'_, AppState>, number: i64) -> Result<(), String> {
    if number < 1 {
        return Err(format!("#{} 메모를 찾을 수 없음", number));
    }
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    memos::delete_by_number(&mut db, Source::App, number)
        .map_err(|_| format!("#{} 메모를 찾을 수 없음", number))
}

#[cfg(test)]
mod tests {
    use super::{PatchField, UpdateMemoByNumberInput, UpdateMemoInput};

    #[test]
    fn update_memo_nullable_fields_distinguish_omitted_null_and_value() {
        let omitted: UpdateMemoInput = serde_json::from_str("{}").unwrap();
        assert_eq!(omitted.project_id, PatchField::Unset);
        assert_eq!(omitted.focus_x, PatchField::Unset);
        assert_eq!(omitted.focus_y, PatchField::Unset);

        let cleared: UpdateMemoInput =
            serde_json::from_str(r#"{"project_id":null,"focus_x":null,"focus_y":null}"#).unwrap();
        assert_eq!(cleared.project_id, PatchField::Null);
        assert_eq!(cleared.focus_x, PatchField::Null);
        assert_eq!(cleared.focus_y, PatchField::Null);

        let updated: UpdateMemoInput =
            serde_json::from_str(r#"{"project_id":42,"focus_x":0.25,"focus_y":0.75}"#).unwrap();
        assert_eq!(updated.project_id, PatchField::Value(42));
        assert_eq!(updated.focus_x, PatchField::Value(0.25));
        assert_eq!(updated.focus_y, PatchField::Value(0.75));
    }

    #[test]
    fn patch_field_maps_to_core_nullable_patch_shape() {
        assert_eq!(PatchField::<i64>::Unset.into_nullable_patch(), None);
        assert_eq!(PatchField::<i64>::Null.into_nullable_patch(), Some(None));
        assert_eq!(
            PatchField::Value(7_i64).into_nullable_patch(),
            Some(Some(7))
        );
    }

    #[test]
    fn update_by_number_input_is_content_only() {
        let input: UpdateMemoByNumberInput = serde_json::from_str(
            r#"{"content":"next","project_id":null,"focus_x":0.5,"tag_names":["later"]}"#,
        )
        .unwrap();
        assert_eq!(input.content.as_deref(), Some("next"));
    }
}
