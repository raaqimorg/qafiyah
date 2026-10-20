use async_trait::async_trait;

use crate::domain::poems::PoemLink;
use crate::domain::{PoetBrief, StoreError, Term};

pub struct Feeling {
    pub id: i32,
    pub parent_id: Option<i32>,
    pub slug: String,
    pub name: String,
    pub wheel_term: Option<String>,
    pub sort_order: i32,
}

pub struct FeelingNode {
    pub id: i32,
    pub slug: String,
    pub name: String,
    pub wheel_term: Option<String>,
    pub parent_slug: Option<String>,
    pub verse_count: i32,
}

pub struct FeelingVerse {
    pub parts: Vec<String>,
    pub position: i32,
    pub feeling: Term,
    pub poem: PoemLink,
    pub poet: PoetBrief,
    pub era: Term,
}

#[async_trait]
pub trait FeelingRepository: Send + Sync {
    async fn all(&self) -> Result<Vec<Feeling>, StoreError>;
    async fn branch_counts(&self) -> Result<Vec<(i32, i32)>, StoreError>;
    async fn verses(
        &self,
        feeling_id: i32,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<FeelingVerse>, StoreError>;
}

fn depth_first<'a>(feelings: &'a [Feeling], parent: Option<i32>, out: &mut Vec<&'a Feeling>) {
    let mut children: Vec<&Feeling> = feelings.iter().filter(|f| f.parent_id == parent).collect();
    children.sort_by_key(|f| f.sort_order);
    for child in children {
        out.push(child);
        depth_first(feelings, Some(child.id), out);
    }
}

pub fn nodes(feelings: &[Feeling], branch_counts: &[(i32, i32)]) -> Vec<FeelingNode> {
    let count = |id: i32| {
        branch_counts
            .iter()
            .find(|(feeling, _)| *feeling == id)
            .map_or(0, |(_, count)| *count)
    };
    let mut ordered = Vec::new();
    depth_first(feelings, None, &mut ordered);
    ordered
        .into_iter()
        .filter(|feeling| count(feeling.id) > 0)
        .map(|feeling| FeelingNode {
            id: feeling.id,
            slug: feeling.slug.clone(),
            name: feeling.name.clone(),
            wheel_term: feeling.wheel_term.clone(),
            parent_slug: feeling
                .parent_id
                .and_then(|parent| feelings.iter().find(|f| f.id == parent))
                .map(|parent| parent.slug.clone()),
            verse_count: count(feeling.id),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feeling(id: i32, parent_id: Option<i32>, slug: &str, sort_order: i32) -> Feeling {
        Feeling {
            id,
            parent_id,
            slug: slug.into(),
            name: slug.into(),
            wheel_term: None,
            sort_order,
        }
    }

    fn tree() -> Vec<Feeling> {
        vec![
            feeling(4, Some(1), "lonely", 2),
            feeling(1, None, "sad", 2),
            feeling(5, Some(4), "isolated", 1),
            feeling(2, None, "happy", 1),
            feeling(3, Some(1), "hurt", 1),
        ]
    }

    fn slugs(nodes: Vec<FeelingNode>) -> Vec<String> {
        nodes.into_iter().map(|n| n.slug).collect()
    }

    #[test]
    fn nodes_come_depth_first_with_siblings_in_their_order() {
        let counts = [(1, 1), (2, 1), (3, 1), (4, 1), (5, 1)];
        assert_eq!(
            slugs(nodes(&tree(), &counts)),
            vec!["happy", "sad", "hurt", "lonely", "isolated"]
        );
    }

    #[test]
    fn a_node_carries_its_branch_count_and_names_its_parent() {
        let counts = [(1, 15), (2, 4), (3, 2), (4, 13), (5, 10)];
        let all = nodes(&tree(), &counts);
        let find = |slug: &str| {
            all.iter()
                .find(|n| n.slug == slug)
                .map(|n| (n.verse_count, n.parent_slug.clone()))
        };
        assert_eq!(find("sad"), Some((15, None)));
        assert_eq!(find("lonely"), Some((13, Some("sad".into()))));
        assert_eq!(find("happy"), Some((4, None)));
    }

    #[test]
    fn a_feeling_with_no_verses_is_left_out() {
        let counts = [(1, 3), (3, 3), (4, 0)];
        assert_eq!(slugs(nodes(&tree(), &counts)), vec!["sad", "hurt"]);
    }
}
