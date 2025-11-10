// this_file: examples/orchestrator_simple.rs
//! Simple demonstration of job orchestration

use haforu::{JobOrchestrator, JobSpec, json_parser::{Job, VariationSetting, FontSpec, ShapingOptions, RenderingOptions, StorageOptions}};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("=== Haforu Job Orchestration Simple Demo ===\n");

    // Create a sample job spec with different patterns
    let mut jobs = Vec::new();

    // Add some jobs with different fonts and variations
    for font_idx in 0..3 {
        for weight in [400.0, 700.0] {
            for text_idx in 0..2 {
                jobs.push(Job {
                    id: format!("job_{}_{}_{}", font_idx, weight as u32, text_idx),
                    font: FontSpec {
                        path: format!("font_{}.ttf", font_idx),
                        variations: if weight != 400.0 {
                            Some(vec![VariationSetting { tag: "wght".to_string(), value: weight }])
                        } else {
                            None
                        },
                        named_instance: None,
                    },
                    text: format!("Sample text {}", text_idx),
                    size: 16.0,
                    shaping: ShapingOptions::default(),
                    rendering: RenderingOptions::default(),
                });
            }
        }
    }

    let spec = JobSpec {
        version: "1.0.0".to_string(),
        jobs,
        storage: StorageOptions::default(),
        include_shaping_output: true,
    };

    // Create orchestrator and analyze jobs
    let orchestrator = JobOrchestrator::new(256)?;
    let stats = orchestrator.analyze_jobs(&spec);

    println!("Job Analysis Results:");
    println!("  Total jobs: {}", stats.total_jobs);
    println!("  Unique fonts: {}", stats.unique_fonts);
    println!("  Unique instances: {}", stats.unique_instances);
    println!("  Avg texts per instance: {:.1}", stats.avg_texts_per_instance);
    println!("  Strategy: {:?}", stats.parallelization_strategy);

    // Create work units
    let work_units = orchestrator.create_work_units(&spec, &stats);
    println!("\nWork units created: {}", work_units.len());

    // Show work unit types
    let mut font_units = 0;
    let mut instance_units = 0;
    let mut text_units = 0;

    for unit in &work_units {
        match unit {
            haforu::orchestrator::WorkUnit::Font { .. } => font_units += 1,
            haforu::orchestrator::WorkUnit::Instance { .. } => instance_units += 1,
            haforu::orchestrator::WorkUnit::Text { .. } => text_units += 1,
        }
    }

    println!("Work unit distribution:");
    println!("  Font-level: {}", font_units);
    println!("  Instance-level: {}", instance_units);
    println!("  Text-level: {}", text_units);

    println!("\n=== Demo Complete ===");
    Ok(())
}