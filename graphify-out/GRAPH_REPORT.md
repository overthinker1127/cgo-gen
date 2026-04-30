# Graph Report - .  (2026-04-30)

## Corpus Check
- 92 files · ~51,046 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 1009 nodes · 2771 edges · 47 communities detected
- Extraction: 65% EXTRACTED · 35% INFERRED · 0% AMBIGUOUS · INFERRED: 977 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- [[_COMMUNITY_C ABI Generation|C ABI Generation]]
- [[_COMMUNITY_Go Facade Rendering|Go Facade Rendering]]
- [[_COMMUNITY_IR Type Normalization|IR Type Normalization]]
- [[_COMMUNITY_Generator Test Contracts|Generator Test Contracts]]
- [[_COMMUNITY_CLI Pipeline Commands|CLI Pipeline Commands]]
- [[_COMMUNITY_Wrapper Source Rendering|Wrapper Source Rendering]]
- [[_COMMUNITY_Config Path Handling|Config Path Handling]]
- [[_COMMUNITY_Counter Example|Counter Example]]
- [[_COMMUNITY_Inventory Example|Inventory Example]]
- [[_COMMUNITY_Pipeline Context|Pipeline Context]]
- [[_COMMUNITY_Compiler Include Discovery|Compiler Include Discovery]]
- [[_COMMUNITY_Simple Fixture Pipeline|Simple Fixture Pipeline]]
- [[_COMMUNITY_Model Ownership Flow|Model Ownership Flow]]
- [[_COMMUNITY_Calculator Example|Calculator Example]]
- [[_COMMUNITY_Score Example|Score Example]]
- [[_COMMUNITY_DataRecord Fixture|DataRecord Fixture]]
- [[_COMMUNITY_Domain Kind Enums|Domain Kind Enums]]
- [[_COMMUNITY_Selected Counter|Selected Counter]]
- [[_COMMUNITY_Shared Dependency|Shared Dependency]]
- [[_COMMUNITY_Selected Widget|Selected Widget]]
- [[_COMMUNITY_DataRecord Files|DataRecord Files]]
- [[_COMMUNITY_Diagnostics Error|Diagnostics Error]]
- [[_COMMUNITY_Field Access Kinds|Field Access Kinds]]
- [[_COMMUNITY_Worker API Fixture|Worker API Fixture]]
- [[_COMMUNITY_String Copy Helper|String Copy Helper]]
- [[_COMMUNITY_Foo Bar Fixture|Foo Bar Fixture]]
- [[_COMMUNITY_Widget Fixture|Widget Fixture]]
- [[_COMMUNITY_Exception Fixture|Exception Fixture]]
- [[_COMMUNITY_Calculator Header|Calculator Header]]
- [[_COMMUNITY_Calculator API|Calculator API]]
- [[_COMMUNITY_Selected Widget Wrapper|Selected Widget Wrapper]]
- [[_COMMUNITY_Selected Counter Wrapper|Selected Counter Wrapper]]
- [[_COMMUNITY_Inventory Item Wrapper|Inventory Item Wrapper]]
- [[_COMMUNITY_Inventory Service Wrapper|Inventory Service Wrapper]]
- [[_COMMUNITY_Counter Wrapper|Counter Wrapper]]
- [[_COMMUNITY_Go Build Flags|Go Build Flags]]
- [[_COMMUNITY_Score Wrapper|Score Wrapper]]
- [[_COMMUNITY_Score API|Score API]]
- [[_COMMUNITY_Type Aliases Header|Type Aliases Header]]
- [[_COMMUNITY_Data Header|Data Header]]
- [[_COMMUNITY_Defs Header|Defs Header]]
- [[_COMMUNITY_Library Root|Library Root]]
- [[_COMMUNITY_Analysis Module|Analysis Module]]
- [[_COMMUNITY_Codegen Module|Codegen Module]]
- [[_COMMUNITY_Domain Module|Domain Module]]
- [[_COMMUNITY_Parsing Module|Parsing Module]]
- [[_COMMUNITY_Pipeline Module|Pipeline Module]]

## God Nodes (most connected - your core abstractions)
1. `normalize()` - 89 edges
2. `parse()` - 85 edges
3. `temp_dir()` - 55 edges
4. `generate_all()` - 54 edges
5. `prepare_config()` - 31 edges
6. `render_go_facade_file()` - 28 edges
7. `normalize_type()` - 25 edges
8. `generate()` - 24 edges
9. `PipelineContext` - 24 edges
10. `normalize_type_with_canonical()` - 22 edges

## Surprising Connections (you probably didn't know these)
- `Documentation README English Copy` --semantically_similar_to--> `cgo-gen`  [INFERRED] [semantically similar]
  docs/README.md → README.md
- `Korean README Localization` --semantically_similar_to--> `cgo-gen`  [INFERRED] [semantically similar]
  docs/README.ko.md → README.md
- `Chinese README Localization` --semantically_similar_to--> `cgo-gen`  [INFERRED] [semantically similar]
  docs/README.zh.md → README.md
- `Japanese README Localization` --semantically_similar_to--> `cgo-gen`  [INFERRED] [semantically similar]
  docs/README.ja.md → README.md
- `Check Then Generate Flow` --references--> `normalize()`  [EXTRACTED]
  README.md → src/codegen/ir_norm.rs

## Hyperedges (group relationships)
- **Simple Fixture End To End Pipeline** — pipeline_config_load, pipeline_parse_stage, pipeline_ir_normalize_stage, pipeline_generator_stage, foo_bar_class, foo_add_function [EXTRACTED 1.00]
- **Translation Unit Selection Policies** — tu_translation_unit_collection, tu_source_preference_policy, tu_grouped_header_expansion, tu_scoped_header_context, tu_headers_only_translation_units [INFERRED 0.86]
- **Model Handle Semantics Suite** — model_projection_modelprojection, context_known_type_lookup, generator_model_handle_semantics, model_out_params_known_model_out_params, compile_smoke_model_borrow_semantics, facade_only_generation_model_facade_compatibility [INFERRED 0.82]
- **Generated Output Verification Suite** — compile_smoke_native_compile_contract, examples_generated_output_checked_in_examples_contract, facade_generate_facade_wrapper_contract, generator_renderer_contracts, model_record_fixture_datarecord_contract, multi_header_generate_multi_header_contract [INFERRED 0.85]
- **Model Record State Access Pattern** — datarecord_datarecord_class, data_tb_data_record, defs_record_field_sizes, types_primitive_aliases, utils_str_copy, datarecord_zero_initialized_storage [EXTRACTED 1.00]
- **CLI Generation Pipeline** — cli_run, config_load_with_raw_clang_args, c_abi_prepare_with_parsed, ir_norm_normalize, c_abi_generate_all, go_facade_render_go_facade [EXTRACTED 1.00]
- **Overload Disambiguation Pattern** — ir_norm_overload_suffix, go_facade_overload_dispatcher, cli_ir_command [INFERRED 0.78]
- **Opaque Ownership And Lifetime Model** — c_abi_opaque_ownership, go_facade_owned_borrowed_handles, model_analysis_projection_detection, ir_norm_ir_module [INFERRED 0.85]
- **Libclang Parsing Flow** — context_pipelinecontext, compiler_translation_unit_discovery, compiler_collect_clang_args, parser_parse, parser_parse_translation_units, parser_collect_entity, parser_parsedapi [EXTRACTED 0.90]

## Communities

### Community 0 - "C ABI Generation"
Cohesion: 0.06
Nodes (140): skips_constructor_for_abstract_class(), C ABI Callback Bridge, class_handles_with_methods(), generate(), generate_all(), Go Package Metadata Generation, opaque_model_value_handles_needing_go_ownership(), prepare_config() (+132 more)

### Community 1 - "Go Facade Rendering"
Cohesion: 0.04
Nodes (144): AnalyzedFacadeClass, base_model_cpp_type(), build_dispatcher(), callback_cgo_param_type(), callback_cgo_return_type(), callback_go_type(), callback_state_name(), callback_state_name_from_function() (+136 more)

### Community 2 - "IR Type Normalization"
Cohesion: 0.04
Nodes (111): Go Overload Dispatcher, alias_primitive_type(), base_model_cpp_type(), by_value_model_params_are_supported(), byte_array_length(), callback_name_set(), canonical_primitive_c_type(), canonicalized_known_record_array_type() (+103 more)

### Community 3 - "Generator Test Contracts"
Cohesion: 0.07
Nodes (67): Abstract Class Constructor Skip Contract, Callback Bridge Facade Contract, Unsupported Declaration Skip Contract, Generator Renderer Contracts, Struct Field Accessor Contract, Default Argument Overload Expansion Contract, Overload Disambiguation Contract, callback_function_type() (+59 more)

### Community 4 - "CLI Pipeline Commands"
Cohesion: 0.04
Nodes (55): generation_headers(), prepare_context(), prepare_with_parsed(), write_ir(), Check Command, Cli, Command, Generate Command (+47 more)

### Community 5 - "Wrapper Source Rendering"
Cohesion: 0.07
Nodes (59): base_model_cpp_type(), call_args(), callback_bridge_functions(), callback_go_export_name(), callback_map(), char_array_length(), existing_owned_model_handles(), exported_cxxflags() (+51 more)

### Community 6 - "Config Path Handling"
Cohesion: 0.06
Nodes (46): temp_dir(), derives_output_filenames_from_header_stem(), directory_wrapper_example_scopes_per_header_output_names(), emits_resolved_ldflags_into_build_flags_go(), EnvGuard, expands_env_tokens_in_clang_args(), expands_env_tokens_in_ldflags(), headers_only_single_header_derives_output_filenames() (+38 more)

### Community 7 - "Counter Example"
Cohesion: 0.05
Nodes (28): clamp_to_zero(), Counter(), increment(), label(), value(), cgowrap_metrics_clamp_to_zero(), cgowrap_metrics_Counter_delete(), cgowrap_metrics_Counter_increment() (+20 more)

### Community 8 - "Inventory Example"
Cohesion: 0.06
Nodes (28): InventoryItem, InventoryService, Id(), InventoryItem, Name(), Quantity(), SetId(), SetName() (+20 more)

### Community 9 - "Pipeline Context"
Cohesion: 0.07
Nodes (34): build_pipeline_context(), collect_known_enum_types(), collect_known_model_types(), Model Borrow And Ownership Compile Semantics, base_cpp_type_name(), enum_cpp_type_name(), Known Model And Enum Type Lookup, Owner Marked Callable Lookup (+26 more)

### Community 10 - "Compiler Include Discovery"
Cohesion: 0.08
Nodes (42): Generated Wrapper Native Compile Contract, add_header_parent_include(), add_parse_entry_parent_include(), add_platform_fallback_includes(), add_platform_fallback_sysroot(), collect_clang_args(), discover_command_output_dir(), discover_linux_driver_include_dirs() (+34 more)

### Community 11 - "Simple Fixture Pipeline"
Cohesion: 0.1
Nodes (27): clash::add Overloads, clash::Widget set Overloads, foo::add Function, foo::Bar Class, foo::Mode Enum, Config Load Stage, Generator Output Stage, IR Normalize Stage (+19 more)

### Community 12 - "Model Ownership Flow"
Cohesion: 0.13
Nodes (16): Multi Header Generation Passes, Opaque Model Value Ownership, 01 C Library Example, 02 C++ Class Example, 03 C++ Inventory Example, Go To C Call Preparation, Owned And Borrowed Go Handles, C Return To Go Mapping (+8 more)

### Community 13 - "Calculator Example"
Cohesion: 0.23
Nodes (9): calculator_add(), calculator_scale(), calculator_subtract(), CalculatorAdd(), CalculatorScale(), CalculatorSubtract(), cgowrap_calculator_add(), cgowrap_calculator_scale() (+1 more)

### Community 14 - "Score Example"
Cohesion: 0.28
Nodes (6): score_delta(), score_total(), cgowrap_score_delta(), cgowrap_score_total(), ScoreDelta(), ScoreTotal()

### Community 15 - "DataRecord Fixture"
Cohesion: 0.53
Nodes (6): TB_DATA_RECORD Struct, DataRecord Class, DataRecord Zero Initialized Storage, Record Field Size Constants, Primitive Type Aliases, Null Safe String Copy Helper

### Community 16 - "Domain Kind Enums"
Cohesion: 0.4
Nodes (4): FieldAccessKind, IrFunctionKind, IrTypeKind, RecordKind

### Community 17 - "Selected Counter"
Cohesion: 0.67
Nodes (1): SelectedCounter

### Community 18 - "Shared Dependency"
Cohesion: 0.67
Nodes (1): SharedDependency

### Community 19 - "Selected Widget"
Cohesion: 0.67
Nodes (1): SelectedWidget

### Community 20 - "DataRecord Files"
Cohesion: 0.67
Nodes (1): DataRecord()

### Community 21 - "Diagnostics Error"
Cohesion: 0.67
Nodes (1): Error

### Community 22 - "Field Access Kinds"
Cohesion: 0.67
Nodes (3): Struct Field Accessor Generation, Field Access Kind Enum, Record Kind Enum

### Community 23 - "Worker API Fixture"
Cohesion: 1.0
Nodes (1): Worker

### Community 24 - "String Copy Helper"
Cohesion: 1.0
Nodes (0): 

### Community 25 - "Foo Bar Fixture"
Cohesion: 1.0
Nodes (1): Bar

### Community 26 - "Widget Fixture"
Cohesion: 1.0
Nodes (1): Widget

### Community 27 - "Exception Fixture"
Cohesion: 1.0
Nodes (2): boom::fail_if Function, boom::Worker Failure Capable API

### Community 28 - "Calculator Header"
Cohesion: 1.0
Nodes (0): 

### Community 29 - "Calculator API"
Cohesion: 1.0
Nodes (0): 

### Community 30 - "Selected Widget Wrapper"
Cohesion: 1.0
Nodes (0): 

### Community 31 - "Selected Counter Wrapper"
Cohesion: 1.0
Nodes (0): 

### Community 32 - "Inventory Item Wrapper"
Cohesion: 1.0
Nodes (0): 

### Community 33 - "Inventory Service Wrapper"
Cohesion: 1.0
Nodes (0): 

### Community 34 - "Counter Wrapper"
Cohesion: 1.0
Nodes (0): 

### Community 35 - "Go Build Flags"
Cohesion: 1.0
Nodes (0): 

### Community 36 - "Score Wrapper"
Cohesion: 1.0
Nodes (0): 

### Community 37 - "Score API"
Cohesion: 1.0
Nodes (0): 

### Community 38 - "Type Aliases Header"
Cohesion: 1.0
Nodes (0): 

### Community 39 - "Data Header"
Cohesion: 1.0
Nodes (0): 

### Community 40 - "Defs Header"
Cohesion: 1.0
Nodes (0): 

### Community 41 - "Library Root"
Cohesion: 1.0
Nodes (0): 

### Community 42 - "Analysis Module"
Cohesion: 1.0
Nodes (0): 

### Community 43 - "Codegen Module"
Cohesion: 1.0
Nodes (0): 

### Community 44 - "Domain Module"
Cohesion: 1.0
Nodes (0): 

### Community 45 - "Parsing Module"
Cohesion: 1.0
Nodes (0): 

### Community 46 - "Pipeline Module"
Cohesion: 1.0
Nodes (0): 

## Ambiguous Edges - Review These
- `Config Load With Raw Clang Args` → `Diagnostics Error Type`  [AMBIGUOUS]
  src/diagnostics.rs · relation: conceptually_related_to

## Knowledge Gaps
- **76 isolated node(s):** `Worker`, `Bar`, `Widget`, `InputConfig`, `RuntimeInputConfig` (+71 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **Thin community `Worker API Fixture`** (2 nodes): `Worker`, `api.hpp`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `String Copy Helper`** (2 nodes): `utils.h`, `strCopy()`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Foo Bar Fixture`** (2 nodes): `Bar`, `foo.hpp`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Widget Fixture`** (2 nodes): `Widget`, `api.hpp`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Exception Fixture`** (2 nodes): `boom::fail_if Function`, `boom::Worker Failure Capable API`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Calculator Header`** (1 nodes): `calculator_wrapper.h`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Calculator API`** (1 nodes): `calculator.h`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Selected Widget Wrapper`** (1 nodes): `selected_widget_wrapper.h`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Selected Counter Wrapper`** (1 nodes): `selected_counter_wrapper.h`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Inventory Item Wrapper`** (1 nodes): `inventory_item_wrapper.h`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Inventory Service Wrapper`** (1 nodes): `inventory_service_wrapper.h`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Counter Wrapper`** (1 nodes): `counter_wrapper.h`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Go Build Flags`** (1 nodes): `build_flags.go`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Score Wrapper`** (1 nodes): `score_wrapper.h`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Score API`** (1 nodes): `score.h`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Type Aliases Header`** (1 nodes): `types.h`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Data Header`** (1 nodes): `data.h`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Defs Header`** (1 nodes): `defs.h`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Library Root`** (1 nodes): `lib.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Analysis Module`** (1 nodes): `mod.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Codegen Module`** (1 nodes): `mod.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Domain Module`** (1 nodes): `mod.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Parsing Module`** (1 nodes): `mod.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Pipeline Module`** (1 nodes): `mod.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **What is the exact relationship between `Config Load With Raw Clang Args` and `Diagnostics Error Type`?**
  _Edge tagged AMBIGUOUS (relation: conceptually_related_to) - confidence is low._
- **Why does `normalize()` connect `C ABI Generation` to `Go Facade Rendering`, `IR Type Normalization`, `CLI Pipeline Commands`, `Pipeline Context`, `Model Ownership Flow`, `Field Access Kinds`?**
  _High betweenness centrality (0.067) - this node is a cross-community bridge._
- **Why does `parse()` connect `C ABI Generation` to `Go Facade Rendering`, `IR Type Normalization`, `Generator Test Contracts`, `CLI Pipeline Commands`, `Wrapper Source Rendering`, `Pipeline Context`, `Compiler Include Discovery`?**
  _High betweenness centrality (0.064) - this node is a cross-community bridge._
- **Why does `generate_all()` connect `C ABI Generation` to `Go Facade Rendering`, `Generator Test Contracts`, `CLI Pipeline Commands`, `Wrapper Source Rendering`, `Config Path Handling`, `Model Ownership Flow`?**
  _High betweenness centrality (0.050) - this node is a cross-community bridge._
- **Are the 61 inferred relationships involving `normalize()` (e.g. with `skips_constructor_for_abstract_class()` and `parses_and_generates_wrapper_for_model_record_fixture()`) actually correct?**
  _`normalize()` has 61 INFERRED edges - model-reasoned connections that need verification._
- **Are the 71 inferred relationships involving `parse()` (e.g. with `skips_constructor_for_abstract_class()` and `parses_and_generates_wrapper_for_model_record_fixture()`) actually correct?**
  _`parse()` has 71 INFERRED edges - model-reasoned connections that need verification._
- **Are the 52 inferred relationships involving `temp_dir()` (e.g. with `temp_dir()` and `temp_test_dir()`) actually correct?**
  _`temp_dir()` has 52 INFERRED edges - model-reasoned connections that need verification._