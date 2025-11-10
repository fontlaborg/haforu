// this_file: examples/orchestrator_demo.rs
//! Demonstration of smart job orchestration for different workload patterns

// this_file: examples/orchestrator_demo.rs
use haforu::{
    JobOrchestrator, JobSpec,
    json_parser::{
        FontSpec, Job, RenderingOptions, ShapingOptions, StorageOptions, VariationSetting,
    },
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("=== Haforu Job Orchestration Demo ===\n");

    // Scenario 1: Many fonts × few instances × few texts
    // Expected: FontLevel parallelization
    demo_many_fonts();

    // Scenario 2: Few fonts × many instances × few texts
    // Expected: InstanceLevel parallelization
    demo_many_instances();

    // Scenario 3: Few fonts × few instances × many texts
    // Expected: TextLevel parallelization
    demo_many_texts();

    // Scenario 4: Balanced workload
    // Expected: Hierarchical parallelization
    demo_balanced();

    Ok(())
}

fn demo_many_fonts() {
    println!("Scenario 1: Many fonts (100) × 2 instances × 5 texts = 1,000 jobs");
    println!("Expected strategy: FontLevel parallelization");
    println!("Rationale: Minimize font loading overhead\n");

    let mut jobs = Vec::new();
    for font_idx in 0..100 {
        for weight in [400.0, 700.0] {
            for text_idx in 0..5 {
                jobs.push(Job {
                    id: format!("job_{}_{}_{}", font_idx, weight as u32, text_idx),
                    font: FontSpec {
                        path: format!("font_{}.ttf", font_idx),
                        variations: Some(vec![VariationSetting {
                            tag: "wght".to_string(),
                            value: weight,
                        }]),
                        named_instance: None,
                    },
                    text: format!("Text {}", text_idx),
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

    let orchestrator = JobOrchestrator::new(512).unwrap();
    let stats = orchestrator.analyze_jobs(&spec);

    println!("Analysis results:");
    println!("  - Unique fonts: {}", stats.unique_fonts);
    println!("  - Unique instances: {}", stats.unique_instances);
    println!(
        "  - Avg texts/instance: {:.1}",
        stats.avg_texts_per_instance
    );
    println!("  - Strategy: {:?}", stats.parallelization_strategy);

    let work_units = orchestrator.create_work_units(&spec, &stats);
    println!("  - Work units created: {}", work_units.len());
    println!();
}

fn demo_many_instances() {
    println!("Scenario 2: 3 fonts × 30 instances × 10 texts = 900 jobs");
    println!("Expected strategy: InstanceLevel parallelization");
    println!("Rationale: Balance between font reuse and parallel efficiency\n");

    let mut jobs = Vec::new();
    for font_idx in 0..3 {
        for weight_step in 0..6 {
            for width_step in 0..5 {
                let weight = 400.0 + (weight_step as f32 * 50.0);
                let width = 75.0 + (width_step as f32 * 10.0);
                for text_idx in 0..10 {
                    jobs.push(Job {
                        id: format!(
                            "job_{}_{}_{}_{}",
                            font_idx, weight as u32, width as u32, text_idx
                        ),
                        font: FontSpec {
                            path: format!("font_{}.ttf", font_idx),
                            variations: Some(vec![
                                VariationSetting {
                                    tag: "wght".to_string(),
                                    value: weight,
                                },
                                VariationSetting {
                                    tag: "wdth".to_string(),
                                    value: width,
                                },
                            ]),
                            named_instance: None,
                        },
                        text: format!("Text {}", text_idx),
                        size: 16.0,
                        shaping: ShapingOptions::default(),
                        rendering: RenderingOptions::default(),
                    });
                }
            }
        }
    }

    let spec = JobSpec {
        version: "1.0.0".to_string(),
        jobs,
        storage: StorageOptions::default(),
        include_shaping_output: true,
    };

    let orchestrator = JobOrchestrator::new(512).unwrap();
    let stats = orchestrator.analyze_jobs(&spec);

    println!("Analysis results:");
    println!("  - Unique fonts: {}", stats.unique_fonts);
    println!("  - Unique instances: {}", stats.unique_instances);
    println!(
        "  - Avg texts/instance: {:.1}",
        stats.avg_texts_per_instance
    );
    println!("  - Strategy: {:?}", stats.parallelization_strategy);

    let work_units = orchestrator.create_work_units(&spec, &stats);
    println!("  - Work units created: {}", work_units.len());
    println!();
}

fn demo_many_texts() {
    println!("Scenario 3: 2 fonts × 3 instances × 500 texts = 3,000 jobs");
    println!("Expected strategy: TextLevel parallelization");
    println!("Rationale: Maximize parallelism for text-heavy workload\n");

    let mut jobs = Vec::new();
    for font_idx in 0..2 {
        for weight in [400.0, 500.0, 700.0] {
            for text_idx in 0..500 {
                jobs.push(Job {
                    id: format!("job_{}_{}_{}", font_idx, weight as u32, text_idx),
                    font: FontSpec {
                        path: format!("font_{}.ttf", font_idx),
                        variations: Some(vec![VariationSetting {
                            tag: "wght".to_string(),
                            value: weight,
                        }]),
                        named_instance: None,
                    },
                    text: format!("Sample text number {} for testing", text_idx),
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

    let orchestrator = JobOrchestrator::new(512).unwrap();
    let stats = orchestrator.analyze_jobs(&spec);

    println!("Analysis results:");
    println!("  - Unique fonts: {}", stats.unique_fonts);
    println!("  - Unique instances: {}", stats.unique_instances);
    println!(
        "  - Avg texts/instance: {:.1}",
        stats.avg_texts_per_instance
    );
    println!("  - Max texts/instance: {}", stats.max_texts_per_instance);
    println!("  - Strategy: {:?}", stats.parallelization_strategy);

    let work_units = orchestrator.create_work_units(&spec, &stats);
    println!("  - Work units created: {}", work_units.len());
    println!();
}

fn demo_balanced() {
    println!("Scenario 4: 10 fonts × 10 instances × 10 texts = 1,000 jobs");
    println!("Expected strategy: Hierarchical parallelization");
    println!("Rationale: Adaptive strategy based on actual distribution\n");

    let mut jobs = Vec::new();
    for font_idx in 0..10 {
        for instance_idx in 0..10 {
            let weight = 400.0 + (instance_idx as f32 * 30.0);
            for text_idx in 0..10 {
                jobs.push(Job {
                    id: format!("job_{}_{}_{}", font_idx, instance_idx, text_idx),
                    font: FontSpec {
                        path: format!("font_{}.ttf", font_idx),
                        variations: Some(vec![VariationSetting {
                            tag: "wght".to_string(),
                            value: weight,
                        }]),
                        named_instance: None,
                    },
                    text: format!("Balanced text {}", text_idx),
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

    let orchestrator = JobOrchestrator::new(512).unwrap();
    let stats = orchestrator.analyze_jobs(&spec);

    println!("Analysis results:");
    println!("  - Unique fonts: {}", stats.unique_fonts);
    println!("  - Unique instances: {}", stats.unique_instances);
    println!(
        "  - Avg texts/instance: {:.1}",
        stats.avg_texts_per_instance
    );
    println!("  - Strategy: {:?}", stats.parallelization_strategy);

    let work_units = orchestrator.create_work_units(&spec, &stats);
    println!("  - Work units created: {}", work_units.len());

    // Show distribution of work unit types in hierarchical mode
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

    if stats.parallelization_strategy == haforu::ParallelizationStrategy::Hierarchical {
        println!("  - Work unit distribution:");
        println!("    - Font-level units: {}", font_units);
        println!("    - Instance-level units: {}", instance_units);
        println!("    - Text-level units: {}", text_units);
    }

    println!("\n=== Demo Complete ===");
}
