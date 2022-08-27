use std::{collections::HashMap, num::NonZeroU64};

use crate::{aggregator::AggregatedData, Log, Process, Trace};
use duo_api as proto;

#[derive(Default)]
pub struct Warehouse {
    // Collection of services.
    services: HashMap<String, Vec<Process>>,
    // <trace_id, Trace>
    traces: HashMap<NonZeroU64, Trace>,
    logs: Vec<Log>,
    // <span_id, Vec<log id>>
    span_log_map: HashMap<NonZeroU64, Vec<usize>>,
}

impl std::fmt::Debug for Warehouse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Warehouse")
            .field("services", &self.services)
            .field("traces", &self.traces.len())
            .field("logs", &self.logs.len())
            .field("span_log_map", &self.span_log_map.len())
            .finish()
    }
}

impl Warehouse {
    pub fn new() -> Self {
        Warehouse::default()
    }

    pub(crate) fn services(&self) -> &HashMap<String, Vec<Process>> {
        &self.services
    }

    pub(crate) fn traces(&self) -> &HashMap<NonZeroU64, Trace> {
        &self.traces
    }

    pub(crate) fn logs(&self) -> &Vec<Log> {
        &self.logs
    }

    pub(crate) fn span_log_map(&self) -> &HashMap<NonZeroU64, Vec<usize>> {
        &self.span_log_map
    }

    /// Register new process and return the process id.
    pub(crate) fn register_process(&mut self, process: proto::Process) -> Process {
        let service_name = process.name;
        let service_processes = self.services.entry(service_name.clone()).or_default();

        // TODO: generate new process id
        let process_id = format!("{}:{}", &service_name, service_processes.len());
        let process = Process {
            id: process_id,
            service_name,
            tags: process.tags,
        };
        service_processes.push(process.clone());
        process
    }

    // Merge aggregated data.
    pub(crate) fn merge_data(&mut self, data: AggregatedData) {
        data.traces.into_iter().for_each(|(id, trace)| {
            self.traces.insert(id, trace);
        });

        // Reserve capacity advanced.
        self.logs.reserve(data.logs.len());
        let base_idx = self.logs.len();
        data.logs.into_iter().enumerate().for_each(|(i, mut log)| {
            let idx = base_idx + i;

            // Exclude those logs without span_id,
            // normally they are not emitted in tracing context.
            if let Some(span_id) = log.span_id {
                let log_idxs = self.span_log_map.entry(span_id).or_default();
                log_idxs.push(idx);
            }

            log.idx = idx;
            self.logs.push(log);
        });
    }
}
