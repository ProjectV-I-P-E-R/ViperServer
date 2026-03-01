use dashmap::{DashMap, DashSet};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::{interval, Duration};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityType {
    Aircraft,
    Vessel,
    SignalNode,
    Orbital,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedObject {
    pub id: String,
    pub callsign: Option<String>,
    pub lat: f64,
    pub lon: f64,
    pub altitude: f32,
    pub heading: f32,
    pub velocity: f32,
    pub entity_type: EntityType,
    pub tags: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityDeltaBatch {
    pub timestamp: u64,
    pub updates: Vec<TrackedObject>,
    pub removals: Vec<String>,
}

pub struct Engine {
    pub objects: DashMap<String, TrackedObject>,
    pub dirty_set: DashSet<String>,
    pub broadcaster: broadcast::Sender<Arc<EntityDeltaBatch>>,
}

impl Engine {
    pub fn new() -> Arc<Self> {
        let (tx, _) = broadcast::channel(1024);
        let engine = Arc::new(Self {
            objects: DashMap::new(),
            dirty_set: DashSet::new(),
            broadcaster: tx,
        });

        let engine_clone = engine.clone();
        tokio::spawn(async move {
            engine_clone.broadcaster_tick().await;
        });

        engine
    }

    pub fn update_object(&self, obj: TrackedObject) {
        let id = obj.id.clone();
        self.objects.insert(id.clone(), obj);
        self.dirty_set.insert(id);
    }

    pub fn remove_object(&self, id: &str) {
        self.objects.remove(id);
    }

    pub fn get_snapshot(&self) -> Vec<TrackedObject> {
        self.objects.iter().map(|ref_multi| ref_multi.value().clone()).collect()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Arc<EntityDeltaBatch>> {
        self.broadcaster.subscribe()
    }

    async fn broadcaster_tick(&self) {
        let mut interval = interval(Duration::from_millis(500));
        loop {
            interval.tick().await;

            if self.dirty_set.is_empty() {
                continue;
            }

            let mut updates = Vec::new();
            
            let dirty_keys: Vec<String> = {
                let mut keys = Vec::new();
                self.dirty_set.retain(|k| {
                    keys.push(k.clone());
                    false
                });
                keys
            };

            for key in dirty_keys {
                if let Some(obj) = self.objects.get(&key) {
                    updates.push(obj.clone());
                }
            }

            if !updates.is_empty() {
                let batch = EntityDeltaBatch {
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64,
                    updates,
                    removals: vec![],
                };

                let _ = self.broadcaster.send(Arc::new(batch));
            }
        }
    }
}