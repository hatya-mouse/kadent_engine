pub mod error;
pub mod node_id;
pub mod topological_sort;

use crate::{
    data_types::{HardwareConfig, ProjectConfig},
    graph::{error::GraphError, node_id::NodeID},
    node::Node,
};
use std::collections::HashMap;

#[derive(Default, Clone)]
pub struct Graph {
    // --- GRAPH STRUCTURE ---
    nodes: HashMap<NodeID, Box<dyn Node>>,
    edges: Vec<(NodeID, usize, NodeID, usize)>,
    adjacency: HashMap<NodeID, Vec<NodeID>>,
    input_id: NodeID,
    output_id: NodeID,

    // --- PROCESSING DATA ---
    sorted_nodes: Vec<NodeID>,
    output_buffers: HashMap<(NodeID, usize), Vec<u8>>,
    // Pointers to the edge buffer in the input order
    node_inputs: HashMap<NodeID, Vec<*const u8>>,
    node_outputs: HashMap<NodeID, Vec<*mut u8>>,
    zero_buffer: Vec<u8>,

    // --- CONFIGURATIONS ---
    /// The project context for the project.
    proj_config: ProjectConfig,
    /// The current hardware context.
    hardware_config: HardwareConfig,

    // --- MISC ---
    next_node_id: u64,
}

impl Graph {
    // --- INITIALIZATION ---

    /// Creates a new Graph instance with the given input and output node..
    pub fn new(
        input_node: Box<dyn Node>,
        output_node: Box<dyn Node>,
        proj_config: ProjectConfig,
        hardware_config: HardwareConfig,
    ) -> Self {
        let mut graph = Graph {
            proj_config,
            hardware_config,
            ..Default::default()
        };
        // Register the input and output nodes
        let input_id = graph.add_node(input_node);
        let output_id = graph.add_node(output_node);
        graph.input_id = input_id;
        graph.output_id = output_id;
        // Return the newly created graph
        graph
    }

    // --- ID GENERATION ---

    /// Sets the next node ID to the given value.
    pub fn set_next_node_id(&mut self, next_node_id: u64) {
        self.next_node_id = next_node_id;
    }

    /// Generates a new NodeID which is unique inside the graph.
    fn generate_node_id(&mut self) -> NodeID {
        let id = NodeID(self.next_node_id);
        self.next_node_id += 1;
        id
    }

    // --- EDGE GETTING ---

    pub fn get_edges(&self) -> &Vec<(NodeID, usize, NodeID, usize)> {
        &self.edges
    }

    // --- NODE GETTING ---

    pub fn get_input_id(&self) -> NodeID {
        self.input_id
    }

    pub fn get_output_id(&self) -> NodeID {
        self.output_id
    }

    pub fn get_node_map(&self) -> &HashMap<NodeID, Box<dyn Node>> {
        &self.nodes
    }

    pub fn get_node_map_mut(&mut self) -> &mut HashMap<NodeID, Box<dyn Node>> {
        &mut self.nodes
    }

    pub fn get_node(&self, id: &NodeID) -> Option<&dyn Node> {
        self.nodes.get(id).map(|track| &**track)
    }

    pub fn get_node_mut(&mut self, id: &NodeID) -> Option<&mut Box<dyn Node>> {
        self.nodes.get_mut(id)
    }

    // --- NODE MANIPULATION ---

    pub fn set_input_id(&mut self, id: NodeID) {
        self.input_id = id;
    }

    pub fn set_output_id(&mut self, id: NodeID) {
        self.output_id = id;
    }

    /// Adds a new node to the graph, and returns the newly generated node ID.
    pub fn add_node(&mut self, mut node: Box<dyn Node>) -> NodeID {
        let id = self.generate_node_id();
        // Update the node
        node.update(&self.proj_config, &self.hardware_config);
        // Insert the node to the map
        self.nodes.insert(id, node);
        id
    }

    /// Adds a new node to the graph with the given ID.
    pub fn add_node_with_id(&mut self, id: NodeID, mut node: Box<dyn Node>) {
        // Update the node
        node.update(&self.proj_config, &self.hardware_config);
        // Insert the node to the map
        self.nodes.insert(id, node);
    }

    /// Removes the node with the given NodeID from the graph.
    pub fn remove_node(&mut self, id: &NodeID) {
        // Remove the edges connected to the node
        self.edges.retain(|edge| edge.0 != *id && edge.2 != *id);
        // Remove the node
        self.nodes.remove(id);
    }

    // --- EDGE MANIPULATION ---

    /// Connects the node's output to another nodes' input without any validation.
    /// Useful for loading the graph from a file, where we assume the file is valid.
    pub fn add_edge_unchecked(&mut self, edge: (NodeID, usize, NodeID, usize)) {
        self.edges.push(edge);
    }

    /// Connects the node's output to another node's input, and returns an error if the type of the output and input are not the same, or if the node is not found.
    pub fn add_edge(&mut self, edge: (NodeID, usize, NodeID, usize)) -> Result<(), GraphError> {
        // Check if the type of the output and input are the same
        let output_type = self
            .nodes
            .get(&edge.0)
            .and_then(|node| node.get_output_type(edge.1))
            .ok_or(GraphError::OutputTypeUnavailable(edge.0, edge.1))?;
        let input_type = self
            .nodes
            .get(&edge.2)
            .and_then(|node| node.get_input_type(edge.3))
            .ok_or(GraphError::InputTypeUnavailable(edge.2, edge.3))?;

        if output_type != input_type {
            return Err(GraphError::NodeTypeMismatch((
                edge.0, edge.1, edge.2, edge.3,
            )));
        }

        self.edges.push(edge);
        Ok(())
    }

    /// Removes the edge from the graph.
    /// Returns an error if the node is not found.
    pub fn remove_edge(&mut self, edge: (NodeID, usize, NodeID, usize)) -> Result<(), GraphError> {
        if let Some(pos) = self.edges.iter().position(|e| *e == edge) {
            self.edges.remove(pos);
            Ok(())
        } else {
            Err(GraphError::EdgeNotFound(edge))
        }
    }

    // --- PROJECT CONTEXT UPDATING ---

    /// Sets the project context to the new one.
    pub fn set_config(&mut self, proj_config: &ProjectConfig, hardware_config: &HardwareConfig) {
        self.proj_config = proj_config.clone();
        self.hardware_config = hardware_config.clone();

        // Call update functions for every nodes
        for node in self.nodes.values_mut() {
            node.update(proj_config, hardware_config);
        }
    }

    // --- GRAPH PROCESSING ---

    /// Prepares the graph for processing. The host must call this function before start processing, or it may lead to undefined behavior.
    pub fn prepare(
        &mut self,
        proj_config: &ProjectConfig,
        hardware_config: &HardwareConfig,
    ) -> Result<(), GraphError> {
        // First sort the graph
        self.sort_graph()?;

        // Allocate output buffer for the input node
        if let Some(input_node) = self.nodes.get_mut(&self.input_id) {
            allocate_output_buffer(
                &self.input_id,
                input_node.as_ref(),
                &mut self.output_buffers,
                &mut self.node_outputs,
                &self.hardware_config,
            )?;
        }

        for node_id in &self.sorted_nodes {
            if let Some(node) = self.nodes.get_mut(node_id) {
                // Call prepare function for every nodes
                node.prepare(proj_config, hardware_config)
                    .map_err(GraphError::NodeError)?;

                allocate_output_buffer(
                    node_id,
                    node.as_ref(),
                    &mut self.output_buffers,
                    &mut self.node_outputs,
                    &self.hardware_config,
                )?;
            }
        }

        // Calculate the max buffer size and create a zero buffer
        let mut max_size = 4usize;
        for (node_id, node) in &self.nodes {
            for i in 0..node.get_input_len() {
                let type_info = node
                    .get_input_type(i)
                    .ok_or(GraphError::InputTypeUnavailable(*node_id, i))?;
                max_size = max_size.max(type_info.size);
            }
        }
        self.zero_buffer = vec![0u8; max_size * self.hardware_config.buffer_size as usize];

        // Build node_inputs from edges
        for edge in &self.edges {
            let Some(ptr) = self
                .output_buffers
                .get(&(edge.0, edge.1))
                .map(|b| b.as_ptr())
            else {
                return Err(GraphError::OutputBufferNotFound(edge.0, edge.1));
            };

            self.node_inputs.entry(edge.2).or_insert_with(|| {
                vec![self.zero_buffer.as_ptr(); self.nodes[&edge.2].get_input_len()]
            })[edge.3] = ptr;
        }

        // For nodes that have no input, set the input buffer to the zero buffer
        let zero_ptr = self.zero_buffer.as_ptr();
        let node_ids_needing_inputs: Vec<NodeID> = self
            .sorted_nodes
            .iter()
            .chain(std::iter::once(&self.output_id))
            .copied()
            .collect();
        for node_id in node_ids_needing_inputs {
            let input_len = self.nodes.get(&node_id).map_or(0, |n| n.get_input_len());
            self.node_inputs
                .entry(node_id)
                .or_insert_with(|| vec![zero_ptr; input_len]);
        }

        Ok(())
    }

    /// Processes the graph in the sorted order and writes the result in the output pointer.
    /// The host must pass the project context which is as the same as the one given in the `set_proj_ctx` function.
    pub fn process(
        &mut self,
        inputs: &[*const u8],
        outputs: &[*mut u8],
        proj_config: &ProjectConfig,
        hardware_config: &HardwareConfig,
    ) {
        // Get the pointer to the output buffer of the input node
        let Some(output_buffers) = self.get_output_ptr(&self.input_id) else {
            return;
        };
        let Some(input_node) = self.nodes.get_mut(&self.input_id) else {
            return;
        };
        // Process the input node
        input_node.process(
            inputs,
            &output_buffers,
            &self.proj_config,
            &self.hardware_config,
        );

        for node_id in self.sorted_nodes.clone() {
            // Get the pointer to the input buffer of the node
            let Some(input_buffers) = self.get_input_ptr(&node_id) else {
                return;
            };
            // Get the pointer to the output buffer of the node
            let Some(output_buffers) = self.get_output_ptr(&node_id) else {
                return;
            };

            // Pass the pointers and process
            if let Some(node) = self.nodes.get_mut(&node_id) {
                node.process(
                    &input_buffers,
                    &output_buffers,
                    &self.proj_config,
                    &self.hardware_config,
                );
            }
        }

        // Get the pointer to the input buffer of the output node
        let Some(input_buffers) = self.get_input_ptr(&self.output_id) else {
            return;
        };
        let Some(output_node) = self.nodes.get_mut(&self.output_id) else {
            return;
        };
        // Process the output node
        // Output data will be written to the output pointer
        output_node.process(&input_buffers, outputs, proj_config, hardware_config);
    }

    fn get_output_ptr(&self, from: &NodeID) -> Option<Vec<*mut u8>> {
        self.node_outputs.get(from).cloned()
    }

    fn get_input_ptr(&self, to: &NodeID) -> Option<Vec<*const u8>> {
        self.node_inputs.get(to).cloned()
    }
}

unsafe impl Send for Graph {}

fn allocate_output_buffer(
    node_id: &NodeID,
    node: &dyn Node,
    output_buffers: &mut HashMap<(NodeID, usize), Vec<u8>>,
    node_outputs: &mut HashMap<NodeID, Vec<*mut u8>>,
    hardware_config: &HardwareConfig,
) -> Result<(), GraphError> {
    // Ensure an output buffer exists even for nodes with no outputs
    node_outputs.entry(*node_id).or_default();
    // Create a buffer for all outputs
    for output_index in 0..node.get_output_len() {
        let output_type = node
            .get_output_type(output_index)
            .ok_or(GraphError::OutputTypeUnavailable(*node_id, output_index))?;
        let buffer = vec![0u8; output_type.size * hardware_config.buffer_size as usize];

        // Insert the output buffer to the output_buffers
        output_buffers.insert((*node_id, output_index), buffer);

        // Register the pointer to the buffer in the node_outputs map
        let Some(ptr) = output_buffers
            .get_mut(&(*node_id, output_index))
            .map(|b| b.as_mut_ptr())
        else {
            return Err(GraphError::OutputBufferNotFound(*node_id, output_index));
        };
        node_outputs.entry(*node_id).or_default().push(ptr);
    }

    Ok(())
}
