// this_file: src/orchestrator.rs
//! Smart job orchestration system for efficient parallel processing
//!
//! This module implements an intelligent job scheduling system that:
//! - Groups jobs by font → instance → text to minimize redundant work
//! - Adapts parallelization strategy based on job distribution
//! - Manages font cache with LRU eviction
//! - Implements work-stealing for load balancing

use crate::error::{Error, Result};
use crate::font_loader::FontLoader;
use crate::json_parser::{JobSpec, VariationSetting};
use log::{debug, info};
use rayon::prelude::*;
use read_fonts::FontRef;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, RwLock};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Statistics about job distribution
#[derive(Debug, Clone)]
pub struct JobStats {
    pub total_jobs: usize,
    pub unique_fonts: usize,
    pub unique_instances: usize,
    pub avg_texts_per_instance: f64,
    pub max_texts_per_instance: usize,
    pub parallelization_strategy: ParallelizationStrategy,
}

/// Strategy for parallelizing work
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParallelizationStrategy {
    /// Many fonts, parallelize at font level
    FontLevel,
    /// Few fonts, many instances, parallelize at instance level
    InstanceLevel,
    /// Few fonts/instances, many texts, parallelize at text level
    TextLevel,
    /// Balanced distribution, use hierarchical parallelization
    Hierarchical,
}

/// A unit of work at different granularities
#[derive(Debug)]
pub enum WorkUnit {
    /// Process all instances and texts for a font
    Font {
        font_path: String,
        instances: Vec<InstanceWork>,
    },
    /// Process all texts for a font instance
    Instance {
        font_path: String,
        instance: InstanceWork,
    },
    /// Process a single text
    Text {
        font_path: String,
        variations: HashMap<String, f32>,
        text: String,
        job_id: String,
    },
}

/// Work for a specific font instance
#[derive(Debug, Clone)]
pub struct InstanceWork {
    pub variations: HashMap<String, f32>,
    pub texts: Vec<(String, String)>, // (text, job_id)
}

/// Font cache with LRU eviction
pub struct FontCache {
    cache: Arc<RwLock<HashMap<String, Arc<Vec<u8>>>>>,
    access_order: Arc<Mutex<VecDeque<String>>>,
    max_size: usize,
    current_size: Arc<RwLock<usize>>,
    // Basic metrics
    hits: AtomicU64,
    misses: AtomicU64,
}

impl FontCache {
    pub fn new(max_size_mb: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            access_order: Arc::new(Mutex::new(VecDeque::new())),
            max_size: max_size_mb * 1024 * 1024,
            current_size: Arc::new(RwLock::new(0)),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    pub fn get(&self, path: &str) -> Option<Arc<Vec<u8>>> {
        let cache = self.cache.read().unwrap();
        if let Some(data) = cache.get(path) {
            // Update access order for LRU
            let mut order = self.access_order.lock().unwrap();
            order.retain(|p| p != path);
            order.push_back(path.to_string());
            self.hits.fetch_add(1, Ordering::Relaxed);
            Some(Arc::clone(data))
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    pub fn insert(&self, path: String, data: Arc<Vec<u8>>) {
        let data_size = data.len();

        // Evict old entries if needed
        while *self.current_size.read().unwrap() + data_size > self.max_size {
            let mut order = self.access_order.lock().unwrap();
            if let Some(evict_path) = order.pop_front() {
                let mut cache = self.cache.write().unwrap();
                if let Some(evicted) = cache.remove(&evict_path) {
                    *self.current_size.write().unwrap() -= evicted.len();
                    debug!("Evicted font from cache: {}", evict_path);
                }
            } else {
                break;
            }
        }

        // Insert new entry
        let mut cache = self.cache.write().unwrap();
        cache.insert(path.clone(), data);
        *self.current_size.write().unwrap() += data_size;

        let mut order = self.access_order.lock().unwrap();
        order.push_back(path);
    }

    /// Return cache hit/miss counters
    pub fn metrics(&self) -> (u64, u64) {
        (
            self.hits.load(Ordering::Relaxed),
            self.misses.load(Ordering::Relaxed),
        )
    }

    pub fn clear(&self) {
        self.cache.write().unwrap().clear();
        self.access_order.lock().unwrap().clear();
        *self.current_size.write().unwrap() = 0;
    }
}

/// Smart job orchestrator
pub struct JobOrchestrator {
    font_loader: Arc<Mutex<FontLoader>>,
    font_cache: Arc<FontCache>,
    max_parallel_fonts: usize,
}

impl JobOrchestrator {
    pub fn new(cache_size_mb: usize) -> Result<Self> {
        Ok(Self {
            font_loader: Arc::new(Mutex::new(FontLoader::new())),
            font_cache: Arc::new(FontCache::new(cache_size_mb)),
            max_parallel_fonts: num_cpus::get().min(8), // Limit concurrent font loads
        })
    }

    /// Analyze job distribution and determine optimal strategy
    pub fn analyze_jobs(&self, spec: &JobSpec) -> JobStats {
        let mut font_instance_texts: HashMap<String, HashMap<String, Vec<String>>> = HashMap::new();

        // Build the job tree: font → instance → texts
        for job in &spec.jobs {
            let instance_key = self.instance_key(&job.font.variations);
            font_instance_texts
                .entry(job.font.path.clone())
                .or_insert_with(HashMap::new)
                .entry(instance_key)
                .or_insert_with(Vec::new)
                .push(job.text.clone());
        }

        let unique_fonts = font_instance_texts.len();
        let total_instances: usize = font_instance_texts.values()
            .map(|instances| instances.len())
            .sum();

        let all_text_counts: Vec<usize> = font_instance_texts.values()
            .flat_map(|instances| instances.values().map(|texts| texts.len()))
            .collect();

        let total_texts: usize = all_text_counts.iter().sum();
        let max_texts = all_text_counts.iter().max().copied().unwrap_or(0);
        let avg_texts = if total_instances > 0 {
            total_texts as f64 / total_instances as f64
        } else {
            0.0
        };

        // Determine parallelization strategy based on distribution
        let strategy = self.determine_strategy(unique_fonts, total_instances, avg_texts, max_texts);

        info!(
            "Job analysis: {} fonts, {} instances, {:.1} avg texts/instance, strategy: {:?}",
            unique_fonts, total_instances, avg_texts, strategy
        );

        JobStats {
            total_jobs: spec.jobs.len(),
            unique_fonts,
            unique_instances: total_instances,
            avg_texts_per_instance: avg_texts,
            max_texts_per_instance: max_texts,
            parallelization_strategy: strategy,
        }
    }

    /// Determine optimal parallelization strategy
    fn determine_strategy(
        &self,
        fonts: usize,
        instances: usize,
        _avg_texts: f64,
        max_texts: usize,
    ) -> ParallelizationStrategy {
        let cores = num_cpus::get();

        // Many fonts relative to cores → parallelize at font level
        if fonts >= cores * 2 {
            return ParallelizationStrategy::FontLevel;
        }

        // Few fonts but many instances → parallelize at instance level
        if fonts < cores / 2 && instances >= cores * 2 {
            return ParallelizationStrategy::InstanceLevel;
        }

        // Few fonts/instances but many texts → parallelize at text level
        if instances < cores && max_texts >= 100 {
            return ParallelizationStrategy::TextLevel;
        }

        // Default to hierarchical for balanced workloads
        ParallelizationStrategy::Hierarchical
    }

    /// Create work units based on parallelization strategy
    pub fn create_work_units(&self, spec: &JobSpec, stats: &JobStats) -> Vec<WorkUnit> {
        let mut work_units = Vec::new();

        // Group jobs by font and instance
        let mut font_tree: HashMap<String, HashMap<String, Vec<(String, String)>>> = HashMap::new();

        for job in &spec.jobs {
            let instance_key = self.instance_key(&job.font.variations);
            font_tree
                .entry(job.font.path.clone())
                .or_insert_with(HashMap::new)
                .entry(instance_key.clone())
                .or_insert_with(Vec::new)
                .push((job.text.clone(), job.id.clone()));
        }

        match stats.parallelization_strategy {
            ParallelizationStrategy::FontLevel => {
                // Create one work unit per font (includes all instances/texts)
                for (font_path, instances) in font_tree {
                    let instance_works: Vec<InstanceWork> = instances.into_iter()
                        .map(|(var_key, texts)| InstanceWork {
                            variations: self.parse_instance_key(&var_key),
                            texts,
                        })
                        .collect();

                    work_units.push(WorkUnit::Font {
                        font_path,
                        instances: instance_works,
                    });
                }
            }

            ParallelizationStrategy::InstanceLevel => {
                // Create one work unit per font instance
                for (font_path, instances) in font_tree {
                    for (var_key, texts) in instances {
                        work_units.push(WorkUnit::Instance {
                            font_path: font_path.clone(),
                            instance: InstanceWork {
                                variations: self.parse_instance_key(&var_key),
                                texts,
                            },
                        });
                    }
                }
            }

            ParallelizationStrategy::TextLevel => {
                // Create one work unit per text
                for (font_path, instances) in font_tree {
                    for (var_key, texts) in instances {
                        let variations = self.parse_instance_key(&var_key);
                        for (text, job_id) in texts {
                            work_units.push(WorkUnit::Text {
                                font_path: font_path.clone(),
                                variations: variations.clone(),
                                text,
                                job_id,
                            });
                        }
                    }
                }
            }

            ParallelizationStrategy::Hierarchical => {
                // Mix strategies based on actual distribution
                for (font_path, instances) in font_tree {
                    if instances.len() >= num_cpus::get() {
                        // Many instances for this font → split by instance
                        for (var_key, texts) in instances {
                            work_units.push(WorkUnit::Instance {
                                font_path: font_path.clone(),
                                instance: InstanceWork {
                                    variations: self.parse_instance_key(&var_key),
                                    texts,
                                },
                            });
                        }
                    } else {
                        // Few instances → keep as single font unit
                        let instance_works: Vec<InstanceWork> = instances.into_iter()
                            .map(|(var_key, texts)| InstanceWork {
                                variations: self.parse_instance_key(&var_key),
                                texts,
                            })
                            .collect();

                        work_units.push(WorkUnit::Font {
                            font_path,
                            instances: instance_works,
                        });
                    }
                }
            }
        }

        info!("Created {} work units with strategy {:?}",
              work_units.len(), stats.parallelization_strategy);

        work_units
    }

    /// Execute work units in parallel
    pub fn execute_work_units(&self, work_units: Vec<WorkUnit>) -> Result<Vec<JobResult>> {
        let start = Instant::now();

        // Process work units in parallel with appropriate strategy
        let results: Vec<Vec<JobResult>> = work_units
            .into_par_iter()
            .map(|unit| self.process_work_unit(unit))
            .collect::<Result<Vec<_>>>()?;

        let flat_results: Vec<JobResult> = results.into_iter().flatten().collect();

        let elapsed = start.elapsed();
        info!(
            "Processed {} results in {:.2}s ({:.1} results/sec)",
            flat_results.len(),
            elapsed.as_secs_f64(),
            flat_results.len() as f64 / elapsed.as_secs_f64()
        );

        Ok(flat_results)
    }

    /// Process a single work unit
    fn process_work_unit(&self, unit: WorkUnit) -> Result<Vec<JobResult>> {
        match unit {
            WorkUnit::Font { font_path, instances } => {
                self.process_font_unit(&font_path, instances)
            }
            WorkUnit::Instance { font_path, instance } => {
                self.process_instance_unit(&font_path, instance)
            }
            WorkUnit::Text { font_path, variations, text, job_id } => {
                self.process_text_unit(&font_path, variations, text, job_id)
                    .map(|result| vec![result])
            }
        }
    }

    /// Process all instances and texts for a font
    fn process_font_unit(
        &self,
        font_path: &str,
        instances: Vec<InstanceWork>,
    ) -> Result<Vec<JobResult>> {
        let font_data = self.load_font_cached(font_path)?;
        let font_ref = FontRef::new(&font_data)
            .map_err(|e| Error::Font(format!("Failed to parse font: {}", e)))?;

        let mut results = Vec::new();

        for instance in instances {
            // Apply variations for this instance
            let shaped_results = self.process_texts_for_instance(
                &font_ref,
                &instance.variations,
                &instance.texts,
            )?;

            for ((text, job_id), shaped) in instance.texts.iter().zip(shaped_results) {
                results.push(JobResult {
                    job_id: job_id.clone(),
                    font_path: font_path.to_string(),
                    variations: instance.variations.clone(),
                    text: text.clone(),
                    shaped_output: shaped,
                    render_ref: None, // Will be filled by renderer
                });
            }
        }

        Ok(results)
    }

    /// Process all texts for a font instance
    fn process_instance_unit(
        &self,
        font_path: &str,
        instance: InstanceWork,
    ) -> Result<Vec<JobResult>> {
        let font_data = self.load_font_cached(font_path)?;
        let font_ref = FontRef::new(&font_data)
            .map_err(|e| Error::Font(format!("Failed to parse font: {}", e)))?;

        let shaped_results = self.process_texts_for_instance(
            &font_ref,
            &instance.variations,
            &instance.texts,
        )?;

        let mut results = Vec::new();
        for ((text, job_id), shaped) in instance.texts.iter().zip(shaped_results) {
            results.push(JobResult {
                job_id: job_id.clone(),
                font_path: font_path.to_string(),
                variations: instance.variations.clone(),
                text: text.clone(),
                shaped_output: shaped,
                render_ref: None,
            });
        }

        Ok(results)
    }

    /// Process a single text
    fn process_text_unit(
        &self,
        font_path: &str,
        variations: HashMap<String, f32>,
        text: String,
        job_id: String,
    ) -> Result<JobResult> {
        let font_data = self.load_font_cached(font_path)?;
        let font_ref = FontRef::new(&font_data)
            .map_err(|e| Error::Font(format!("Failed to parse font: {}", e)))?;

        let shaped = self.shape_single_text(&font_ref, &variations, &text)?;

        Ok(JobResult {
            job_id,
            font_path: font_path.to_string(),
            variations,
            text,
            shaped_output: shaped,
            render_ref: None,
        })
    }

    /// Load font with caching
    fn load_font_cached(&self, path: &str) -> Result<Arc<Vec<u8>>> {
        // Check cache first
        if let Some(data) = self.font_cache.get(path) {
            debug!("Font cache hit: {}", path);
            return Ok(data);
        }

        // Load font
        debug!("Loading font: {}", path);
        let mut loader = self.font_loader.lock().unwrap();
        let data = loader.load_font_data(path)?;

        // Cache for future use
        self.font_cache.insert(path.to_string(), Arc::clone(&data));

        Ok(data)
    }

    /// Process multiple texts for a font instance
    fn process_texts_for_instance(
        &self,
        _font_ref: &FontRef,
        _variations: &HashMap<String, f32>,
        texts: &[(String, String)],
    ) -> Result<Vec<String>> {
        // TODO: Implement actual shaping with HarfRust
        // For now, return placeholder
        Ok(texts.iter().map(|(text, _)| {
            format!("shaped:{}", text.len())
        }).collect())
    }

    /// Shape a single text
    fn shape_single_text(
        &self,
        _font_ref: &FontRef,
        _variations: &HashMap<String, f32>,
        text: &str,
    ) -> Result<String> {
        // TODO: Implement actual shaping with HarfRust
        Ok(format!("shaped:{}", text.len()))
    }

    /// Create instance key for grouping
    fn instance_key(&self, variations: &Option<Vec<VariationSetting>>) -> String {
        if let Some(vars) = variations {
            let mut items: Vec<_> = vars.iter()
                .map(|v| (&v.tag, v.value))
                .collect();
            items.sort_by_key(|&(k, _)| k);

            items.iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join(",")
        } else {
            String::new()
        }
    }

    /// Convert variations to HashMap for internal use
    fn variations_to_map(variations: &Option<Vec<VariationSetting>>) -> HashMap<String, f32> {
        if let Some(vars) = variations {
            vars.iter()
                .map(|v| (v.tag.clone(), v.value))
                .collect()
        } else {
            HashMap::new()
        }
    }

    /// Parse instance key back to variations
    fn parse_instance_key(&self, key: &str) -> HashMap<String, f32> {
        if key.is_empty() {
            return HashMap::new();
        }

        key.split(',')
            .filter_map(|part| {
                let mut split = part.split('=');
                match (split.next(), split.next()) {
                    (Some(k), Some(v)) => v.parse::<f32>().ok().map(|val| (k.to_string(), val)),
                    _ => None,
                }
            })
            .collect()
    }
}

/// Result of processing a job
#[derive(Debug, Clone)]
pub struct JobResult {
    pub job_id: String,
    pub font_path: String,
    pub variations: HashMap<String, f32>,
    pub text: String,
    pub shaped_output: String,
    pub render_ref: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json_parser::Job;
    use crate::json_parser::{FontSpec, ShapingOptions, RenderingOptions};

    fn create_test_job(id: &str, font_path: &str, text: &str, variations: Option<Vec<VariationSetting>>) -> Job {
        Job {
            id: id.to_string(),
            font: FontSpec {
                path: font_path.to_string(),
                variations,
                named_instance: None,
            },
            text: text.to_string(),
            size: 16.0,
            shaping: ShapingOptions::default(),
            rendering: RenderingOptions::default(),
        }
    }

    #[test]
    fn test_job_analysis() {
        let spec = JobSpec {
            version: "1.0.0".to_string(),
            jobs: vec![
                create_test_job("job1", "font1.ttf", "Hello", None),
                create_test_job("job2", "font1.ttf", "World", None),
                create_test_job("job3", "font1.ttf", "Bold", Some(vec![
                    VariationSetting { tag: "wght".to_string(), value: 700.0 }
                ])),
                create_test_job("job4", "font2.ttf", "Different", None),
            ],
            storage: crate::json_parser::StorageOptions::default(),
            include_shaping_output: true,
        };

        let orchestrator = JobOrchestrator::new(256).unwrap();
        let stats = orchestrator.analyze_jobs(&spec);

        assert_eq!(stats.total_jobs, 4);
        assert_eq!(stats.unique_fonts, 2);
        assert_eq!(stats.unique_instances, 3);
    }

    #[test]
    fn test_instance_key() {
        let orchestrator = JobOrchestrator::new(256).unwrap();

        let variations = Some(vec![
            VariationSetting { tag: "wght".to_string(), value: 700.0 },
            VariationSetting { tag: "wdth".to_string(), value: 100.0 },
        ]);

        let key = orchestrator.instance_key(&variations);
        assert_eq!(key, "wdth=100,wght=700"); // Sorted alphabetically

        let parsed = orchestrator.parse_instance_key(&key);
        let expected = JobOrchestrator::variations_to_map(&variations);
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_parallelization_strategy() {
        let orchestrator = JobOrchestrator::new(256).unwrap();
        let cores = num_cpus::get();

        // Many fonts → FontLevel
        let strategy = orchestrator.determine_strategy(cores * 3, cores, 10.0, 20);
        assert_eq!(strategy, ParallelizationStrategy::FontLevel);

        // Few fonts, many instances → InstanceLevel
        let strategy = orchestrator.determine_strategy(2, cores * 3, 10.0, 20);
        assert_eq!(strategy, ParallelizationStrategy::InstanceLevel);

        // Few fonts/instances, many texts → TextLevel
        let strategy = orchestrator.determine_strategy(2, 3, 150.0, 200);
        assert_eq!(strategy, ParallelizationStrategy::TextLevel);
    }

    #[test]
    fn test_font_cache_lru() {
        let cache = FontCache::new(1); // 1 MB cache

        // Insert first font (500 KB)
        let data1 = Arc::new(vec![0u8; 500_000]);
        cache.insert("font1.ttf".to_string(), Arc::clone(&data1));

        // Insert second font (600 KB) - should evict first
        let data2 = Arc::new(vec![1u8; 600_000]);
        cache.insert("font2.ttf".to_string(), Arc::clone(&data2));

        // First font should be evicted
        assert!(cache.get("font1.ttf").is_none());
        assert!(cache.get("font2.ttf").is_some());
    }

    #[test]
    fn test_font_cache_metrics_hit_miss() {
        let cache = FontCache::new(1); // 1 MB
        let (h0, m0) = cache.metrics();
        assert_eq!(h0, 0);
        assert_eq!(m0, 0);

        // Miss on empty cache
        assert!(cache.get("/nope.ttf").is_none());
        let (h1, m1) = cache.metrics();
        assert_eq!(h1, 0, "hits should stay 0 after miss");
        assert_eq!(m1, 1, "one miss recorded");

        // Insert and then hit
        let path = "/font.ttf".to_string();
        cache.insert(path.clone(), Arc::new(vec![1u8; 16]));
        assert!(cache.get(&path).is_some());
        let (h2, m2) = cache.metrics();
        assert_eq!(h2, 1, "one hit recorded");
        assert_eq!(m2, 1, "miss count unchanged");
    }
}
