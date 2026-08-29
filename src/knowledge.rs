//! テトリス知識体系（1,160+項目）および地形パターン管理モジュール (addplan3.md)

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// 構造化知識アイテム
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeItem {
    pub id: String,
    pub category: String,
    pub name: String,
    pub description: String,
    pub importance: f32,
    pub higher_is_better: bool,
    pub related_features: Vec<String>,
    pub ai_usage: Vec<String>,
    pub source: Vec<String>,
}

/// 構造化地形パターン
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainPattern {
    pub id: String,
    pub name: String,
    pub category: String,
    #[serde(default)]
    pub notch_cols: Vec<usize>,
    #[serde(default)]
    pub roof_col: usize,
    #[serde(default)]
    pub recommended_cols: Vec<usize>,
    #[serde(default)]
    pub height_depth: usize,
    #[serde(default)]
    pub expected_attack: usize,
    #[serde(default)]
    pub b2b: bool,
}

/// 知識ベース全体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeBase {
    pub total_items: usize,
    pub version: String,
    pub items: Vec<KnowledgeItem>,
}

impl KnowledgeBase {
    /// デフォルトパス（`tetris-ai-research/11_dataset/knowledge.json`）から知識ベースを読み込む
    pub fn load_from_default_path() -> Result<Self, Box<dyn std::error::Error>> {
        let path = Path::new("tetris-ai-research/11_dataset/knowledge.json");
        let mut file = File::open(path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        let kb: KnowledgeBase = serde_json::from_str(&content)?;
        Ok(kb)
    }

    /// カテゴリ別にアイテムを抽出
    pub fn get_by_category(&self, category: &str) -> Vec<&KnowledgeItem> {
        self.items.iter().filter(|it| it.category == category).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_knowledge_base_items() {
        let kb = KnowledgeBase::load_from_default_path();
        assert!(kb.is_ok(), "KnowledgeBase should load successfully from tetris-ai-research/11_dataset/knowledge.json");
        let kb = kb.unwrap();
        assert!(kb.total_items >= 1160, "KnowledgeBase must contain at least 1,160 items, got {}", kb.total_items);
        assert_eq!(kb.items.len(), kb.total_items);

        let tspin_items = kb.get_by_category("tspin");
        assert!(!tspin_items.is_empty(), "T-Spin category must have items");

        let terrain_items = kb.get_by_category("terrain");
        assert!(!terrain_items.is_empty(), "Terrain category must have items");
    }
}
