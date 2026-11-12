use egui_node_graph::*;
use std::borrow::Cow;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DataType {
    Trigger,     // Hilo de ejecución (cable blanco)
    Condition,   // Booleano (cable verde)
    Numeric,     // Valores de tiempo o IA (cable azul)
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum ValueType {
    Trigger,
    Condition(bool),
    Numeric(i32),
}

impl Default for ValueType {
    fn default() -> Self {
        ValueType::Trigger
    }
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NodeGraphCustomState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrickResponse;

impl UserResponseTrait for FrickResponse {}

impl DataTypeTrait<NodeGraphCustomState> for DataType {
    fn data_type_color(&self, _user_state: &mut NodeGraphCustomState) -> egui::Color32 {
        match self {
            DataType::Trigger => egui::Color32::WHITE,
            DataType::Condition => egui::Color32::from_rgb(50, 200, 100),
            DataType::Numeric => egui::Color32::from_rgb(100, 150, 255),
        }
    }
    fn name(&self) -> Cow<'_, str> {
        match self {
            DataType::Trigger => Cow::Borrowed("Ejecución"),
            DataType::Condition => Cow::Borrowed("Condición"),
            DataType::Numeric => Cow::Borrowed("Número"),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum NodeTemplate {
    // Eventos (Rojos)
    EventOnNightStart,
    EventOnWindowKnock,
    EventOnBlindStateChange,
    
    // Acciones (Azules)
    ActionTriggerJumpscare,
    ActionHoldBlindOpen,
    ActionPlayAudio,
    ActionChangeState,
    
    // Condiciones (Verdes)
    ConditionIsFlashlightOn,
    ConditionIsBlindClosed,
    ConditionIsAccessoryRemoved,
}

impl NodeDataTrait for NodeTemplate {
    type Response = FrickResponse;
    type UserState = NodeGraphCustomState;
    type DataType = DataType;
    type ValueType = ValueType;

    fn bottom_ui(
        &self,
        _ui: &mut egui::Ui,
        _node_id: NodeId,
        _graph: &Graph<Self, Self::DataType, Self::ValueType>,
        _user_state: &mut Self::UserState,
    ) -> Vec<egui_node_graph::NodeResponse<Self::Response, Self>> {
        Vec::new()
    }
}

impl WidgetValueTrait for ValueType {
    type Response = FrickResponse;
    type UserState = NodeGraphCustomState;
    type NodeData = NodeTemplate;

    fn value_widget(
        &mut self,
        _param_name: &str,
        _node_id: NodeId,
        ui: &mut egui::Ui,
        _user_state: &mut Self::UserState,
        _node_data: &Self::NodeData,
    ) -> Vec<Self::Response> {
        match self {
            ValueType::Numeric(val) => {
                ui.add(egui::DragValue::new(val));
            }
            ValueType::Condition(b) => {
                ui.checkbox(b, "");
            }
            _ => {}
        }
        Vec::new()
    }
}

impl NodeTemplateTrait for NodeTemplate {
    type NodeData = Self;
    type DataType = DataType;
    type ValueType = ValueType;
    type UserState = NodeGraphCustomState;
    type CategoryType = &'static str;

    fn node_finder_label(&self, _user_state: &mut Self::UserState) -> Cow<'_, str> {
        match self {
            Self::EventOnNightStart => Cow::Borrowed("Evento: Al Iniciar Noche"),
            Self::EventOnWindowKnock => Cow::Borrowed("Evento: Al Llegar a Ventana"),
            Self::EventOnBlindStateChange => Cow::Borrowed("Evento: Al Mover Persiana"),
            Self::ActionTriggerJumpscare => Cow::Borrowed("Acción: Ejecutar Jumpscare"),
            Self::ActionHoldBlindOpen => Cow::Borrowed("Acción: Forzar Persiana Abierta"),
            Self::ActionPlayAudio => Cow::Borrowed("Acción: Reproducir Sonido"),
            Self::ActionChangeState => Cow::Borrowed("Acción: Cambiar Estado"),
            Self::ConditionIsFlashlightOn => Cow::Borrowed("Condición: ¿Linterna Encendida?"),
            Self::ConditionIsBlindClosed => Cow::Borrowed("Condición: ¿Persiana Cerrada?"),
            Self::ConditionIsAccessoryRemoved => Cow::Borrowed("Condición: ¿Gafas Retiradas?"),
        }
    }

    fn node_finder_categories(&self, _user_state: &mut Self::UserState) -> Vec<Self::CategoryType> {
        match self {
            Self::EventOnNightStart | Self::EventOnWindowKnock | Self::EventOnBlindStateChange => {
                vec!["Eventos"]
            }
            Self::ActionTriggerJumpscare
            | Self::ActionHoldBlindOpen
            | Self::ActionPlayAudio
            | Self::ActionChangeState => vec!["Acciones"],
            Self::ConditionIsFlashlightOn
            | Self::ConditionIsBlindClosed
            | Self::ConditionIsAccessoryRemoved => vec!["Condiciones"],
        }
    }

    fn node_graph_label(&self, user_state: &mut Self::UserState) -> String {
        self.node_finder_label(user_state).into_owned()
    }

    fn user_data(&self, _user_state: &mut Self::UserState) -> Self::NodeData {
        self.clone()
    }

    fn build_node(
        &self,
        graph: &mut Graph<Self::NodeData, Self::DataType, Self::ValueType>,
        _user_state: &mut Self::UserState,
        node_id: NodeId,
    ) {
        match self {
            Self::EventOnNightStart => {
                graph.add_output_param(node_id, "Iniciar".into(), DataType::Trigger);
            }
            Self::EventOnWindowKnock => {
                graph.add_output_param(node_id, "Al Llegar".into(), DataType::Trigger);
            }
            Self::EventOnBlindStateChange => {
                graph.add_output_param(node_id, "Al Cambiar".into(), DataType::Trigger);
            }
            Self::ConditionIsBlindClosed => {
                graph.add_input_param(
                    node_id,
                    "Evaluar".into(),
                    DataType::Trigger,
                    ValueType::Trigger,
                    InputParamKind::ConnectionOnly,
                    true,
                );
                graph.add_output_param(node_id, "Verdadero".into(), DataType::Condition);
                graph.add_output_param(node_id, "Falso".into(), DataType::Condition);
            }
            Self::ConditionIsFlashlightOn => {
                graph.add_input_param(
                    node_id,
                    "Evaluar".into(),
                    DataType::Trigger,
                    ValueType::Trigger,
                    InputParamKind::ConnectionOnly,
                    true,
                );
                graph.add_output_param(node_id, "Verdadero".into(), DataType::Condition);
                graph.add_output_param(node_id, "Falso".into(), DataType::Condition);
            }
            Self::ConditionIsAccessoryRemoved => {
                graph.add_input_param(
                    node_id,
                    "Evaluar".into(),
                    DataType::Trigger,
                    ValueType::Trigger,
                    InputParamKind::ConnectionOnly,
                    true,
                );
                graph.add_output_param(node_id, "Verdadero".into(), DataType::Condition);
                graph.add_output_param(node_id, "Falso".into(), DataType::Condition);
            }
            Self::ActionTriggerJumpscare => {
                graph.add_input_param(
                    node_id,
                    "Ejecutar".into(),
                    DataType::Trigger,
                    ValueType::Trigger,
                    InputParamKind::ConnectionOnly,
                    true,
                );
            }
            Self::ActionHoldBlindOpen => {
                graph.add_input_param(
                    node_id,
                    "Ejecutar".into(),
                    DataType::Trigger,
                    ValueType::Trigger,
                    InputParamKind::ConnectionOnly,
                    true,
                );
            }
            Self::ActionPlayAudio => {
                graph.add_input_param(
                    node_id,
                    "Ejecutar".into(),
                    DataType::Trigger,
                    ValueType::Trigger,
                    InputParamKind::ConnectionOnly,
                    true,
                );
                graph.add_output_param(node_id, "Al Terminar".into(), DataType::Trigger);
            }
            Self::ActionChangeState => {
                graph.add_input_param(
                    node_id,
                    "Ejecutar".into(),
                    DataType::Trigger,
                    ValueType::Trigger,
                    InputParamKind::ConnectionOnly,
                    true,
                );
                graph.add_output_param(node_id, "Al Cambiar".into(), DataType::Trigger);
            }
        }
    }
}

pub type FrickGraph = Graph<NodeTemplate, DataType, ValueType>;
pub type FrickEditorState = GraphEditorState<NodeTemplate, DataType, ValueType, NodeTemplate, NodeGraphCustomState>;

#[derive(Clone, Copy, Debug)]
pub struct AllMyNodeTemplates;

impl NodeTemplateIter for AllMyNodeTemplates {
    type Item = NodeTemplate;

    fn all_kinds(&self) -> Vec<Self::Item> {
        vec![
            NodeTemplate::EventOnNightStart,
            NodeTemplate::EventOnWindowKnock,
            NodeTemplate::EventOnBlindStateChange,
            NodeTemplate::ConditionIsFlashlightOn,
            NodeTemplate::ConditionIsBlindClosed,
            NodeTemplate::ConditionIsAccessoryRemoved,
            NodeTemplate::ActionTriggerJumpscare,
            NodeTemplate::ActionHoldBlindOpen,
            NodeTemplate::ActionPlayAudio,
            NodeTemplate::ActionChangeState,
        ]
    }
}

pub fn draw_node_graph_ui(
    ui: &mut egui::Ui,
    state: &mut FrickEditorState,
    custom_state: &mut NodeGraphCustomState,
) {
    let _response = state.draw_graph_editor(
        ui,
        AllMyNodeTemplates,
        custom_state,
        Vec::default(),
    );
}

/// Crea un grafo inicial con la lógica por defecto de Murray Moose
pub fn create_default_murray_graph() -> (FrickGraph, FrickEditorState) {
    let mut state = FrickEditorState::default();
    let mut custom = NodeGraphCustomState::default();

    // Evento: Al llegar a ventana
    let knock_node = state.graph.add_node(
        "Evento: Al Llegar a Ventana".into(),
        NodeTemplate::EventOnWindowKnock,
        |graph, node_id| {
            NodeTemplate::EventOnWindowKnock.build_node(graph, &mut custom, node_id);
        },
    );

    // Condición: ¿Persiana cerrada?
    let blind_cond = state.graph.add_node(
        "Condición: ¿Persiana Cerrada?".into(),
        NodeTemplate::ConditionIsBlindClosed,
        |graph, node_id| {
            NodeTemplate::ConditionIsBlindClosed.build_node(graph, &mut custom, node_id);
        },
    );

    // Acción: Jumpscare
    let scare_node = state.graph.add_node(
        "Acción: Ejecutar Jumpscare".into(),
        NodeTemplate::ActionTriggerJumpscare,
        |graph, node_id| {
            NodeTemplate::ActionTriggerJumpscare.build_node(graph, &mut custom, node_id);
        },
    );

    // Conectar knock -> eval blind
    if let (Some(out_param), Some(in_param)) = (
        state.graph[knock_node].get_output("Al Llegar").ok(),
        state.graph[blind_cond].get_input("Evaluar").ok(),
    ) {
        state.graph.add_connection(out_param, in_param, 0);
    }

    // Conectar blind falso (abierta) -> jumpscare
    if let (Some(out_param), Some(in_param)) = (
        state.graph[blind_cond].get_output("Falso").ok(),
        state.graph[scare_node].get_input("Ejecutar").ok(),
    ) {
        state.graph.add_connection(out_param, in_param, 0);
    }

    // Sincronizar node_order para evitar el panic de egui_node_graph2
    state.node_order = state.graph.iter_nodes().collect();

    let graph = state.graph.clone();
    (graph, state)
}

/// Traduce el grafo de nodos a código GameMaker VM (GML)
pub fn generate_gml_from_graph(graph: &FrickGraph, animatronic_id: &str) -> String {
    let mut gml = String::new();
    gml.push_str(&format!(
        "// ==============================================================\n\
         // LOGICA DE COMPORTAMIENTO (GENERADA DESDE GRAFO DE NODOS)\n\
         // Animatronico: {}\n\
         // ==============================================================\n\n",
        animatronic_id
    ));

    // Analizar nodos presentes en el grafo
    let mut has_knock_event = false;
    let mut has_blind_condition = false;
    let mut has_scare_action = false;
    let mut has_flashlight_condition = false;
    let mut has_accessory_condition = false;

    for node_id in graph.iter_nodes() {
        let node = &graph[node_id];
        match node.user_data {
            NodeTemplate::EventOnWindowKnock => has_knock_event = true,
            NodeTemplate::ConditionIsBlindClosed => has_blind_condition = true,
            NodeTemplate::ActionTriggerJumpscare => has_scare_action = true,
            NodeTemplate::ConditionIsFlashlightOn => has_flashlight_condition = true,
            NodeTemplate::ConditionIsAccessoryRemoved => has_accessory_condition = true,
            _ => {}
        }
    }

    gml.push_str("// --- 1. SEGURIDAD DE CICLO DE VIDA (GEMINI.md Regla #2) ---\n");
    gml.push_str("if (!variable_instance_exists(id, \"attack_timer\")) attack_timer = 0;\n");
    gml.push_str("if (!variable_instance_exists(id, \"attack_max_time\")) attack_max_time = 320;\n\n");

    if has_knock_event || has_blind_condition {
        gml.push_str("// --- 2. EVALUACION DE VENTANAS Y PERSIANAS (GEMINI.md Regla #3) ---\n");
        gml.push_str("if (variable_instance_exists(id, \"my_blind\") && instance_exists(my_blind)) {\n");
        gml.push_str("    if (!my_blind.Open) {\n");
        gml.push_str("        // Persiana cerrada por el jugador: Murray NO puede atacar ni forzar apertura\n");
        gml.push_str("        attack_timer = max(0, attack_timer - 1);\n");
        gml.push_str("    } else {\n");
        gml.push_str("        // Persiana levantada: el temporizador de ataque avanza\n");
        gml.push_str("        attack_timer += 1;\n");
        if has_scare_action {
            gml.push_str("        if (attack_timer >= attack_max_time) {\n");
            gml.push_str("            // Disparar jumpscare seguro\n");
            gml.push_str("            if (state == 1 || state == 11) {\n");
            gml.push_str("                state = 4;\n");
            gml.push_str("                alarm[0] = 30;\n");
            gml.push_str("            }\n");
            gml.push_str("        }\n");
        }
        gml.push_str("    }\n");
        gml.push_str("}\n\n");
    }

    if has_flashlight_condition || has_accessory_condition {
        gml.push_str("// --- 3. EVALUACION DE DEFENSAS (LINTERNA Y RETIRO DE ACCESORIOS) ---\n");
        gml.push_str("if (state == 11) {\n");
        gml.push_str("    // Estado 11: Asustado sin lentes/accesorio tras clic del jugador\n");
        gml.push_str("    if (variable_global_exists(\"FlashlightOn\") && global.FlashlightOn) {\n");
        gml.push_str("        // Linterna activa: Derrota cómica del animatrónico (Estado 2)\n");
        gml.push_str("        state = 2;\n");
        gml.push_str("        defeat_timer = 0;\n");
        gml.push_str("        var _snd = asset_get_index(\"snd_defeat\");\n");
        gml.push_str("        if (_snd != -1) audio_play_sound(_snd, 10, false);\n");
        gml.push_str("    }\n");
        gml.push_str("}\n");
    }

    gml
}

