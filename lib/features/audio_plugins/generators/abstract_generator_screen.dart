import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:karbeat/features/components/plugin_parameter_widget.dart';
import 'package:karbeat/src/rust/api/plugin.dart' as plugin_api; // Import the universal widgets

/// # Overview
/// 
/// **Abstract base class for generator plugin screens.**
///
/// Provides default implementations for:
/// - Parameter polling (from audio thread feedback)
/// - Loading parameter specs
/// - Setting parameters (optimistic UI + backend)
/// - Standard Scaffold/AppBar layout
/// - Automatic Dynamic UI Generation based on Rust #[param] specs
///
/// Subclasses can override [buildGeneratorBody] to define a custom effect UI, 
/// but it defaults to an automatically generated layout.
abstract class AbstractGeneratorScreen extends ConsumerStatefulWidget {
  final int generatorId;

  const AbstractGeneratorScreen({super.key, required this.generatorId});
}

abstract class AbstractGeneratorScreenState<T extends AbstractGeneratorScreen>
    extends ConsumerState<T> {
  List<plugin_api.UiPluginParameter> parameters = [];
  bool isLoading = true;
  String? errorMessage;

  Timer? _parameterPollTimer;

  String get generatorName => 'Generator';

  /// Helper getter to automatically group parameters by their Rust `group` string.
  Map<String, List<plugin_api.UiPluginParameter>> get groupedParameters {
    final map = <String, List<plugin_api.UiPluginParameter>>{};
    for (final param in parameters) {
      if (!map.containsKey(param.group)) {
        map[param.group] = [];
      }
      map[param.group]!.add(param);
    }
    return map;
  }

  @override
  void initState() {
    super.initState();
    loadParameterSpecs();
    startParameterPolling();
  }

  @override
  void dispose() {
    _parameterPollTimer?.cancel();
    super.dispose();
  }

  @protected
  void startParameterPolling() async {
    await plugin_api.queryGeneratorParameters(generatorId: widget.generatorId);

    _parameterPollTimer = Timer.periodic(
      const Duration(milliseconds: 50),
      (_) => _pollParameterFeedback(),
    );
  }

  void _pollParameterFeedback() async {
    if (!mounted) return;

    try {
      final snapshots = await plugin_api.pollGeneratorParameterFeedback();
      if (snapshots.isEmpty) return;

      await plugin_api.syncGeneratorParametersFromAudio(snapshots: snapshots);

      bool updated = false;

      setState(() {
        for (final snapshot in snapshots) {
          if (snapshot.generatorId != widget.generatorId) continue;

          for (final paramValue in snapshot.parameters) {
            final index = parameters.indexWhere(
              (p) => p.id == paramValue.paramId,
            );
            if (index != -1) {
              // Copy all immutable fields but update the value
              parameters[index] = plugin_api.UiPluginParameter(
                id: parameters[index].id,
                name: parameters[index].name,
                group: parameters[index].group,
                value: paramValue.value,
                min: parameters[index].min,
                max: parameters[index].max,
                defaultValue: parameters[index].defaultValue,
                step: parameters[index].step,
                paramType: parameters[index].paramType,
                choices: parameters[index].choices,
              );
              updated = true;
            }
          }
        }
      });

      if (updated) {
        onParametersUpdated();
      }
    } catch (e) {
      debugPrint('Error polling generator parameter feedback: $e');
    }
  }

  @protected
  Future<void> loadParameterSpecs() async {
    try {
      final specs = await plugin_api.getGeneratorParameterSpecs(
        generatorId: widget.generatorId,
      );
      setState(() {
        // Sort parameters by ID to maintain a consistent UI order
        specs.sort((a, b) => a.id.compareTo(b.id));
        parameters = specs;
        isLoading = false;
      });
      onParametersUpdated();
    } catch (e) {
      setState(() {
        errorMessage = 'Failed to load generator parameters: $e';
        isLoading = false;
      });
    }
  }

  @protected
  Future<void> setParameter(int paramId, double value) async {
    setState(() {
      final index = parameters.indexWhere((p) => p.id == paramId);
      if (index != -1) {
        parameters[index] = plugin_api.UiPluginParameter(
          id: parameters[index].id,
          name: parameters[index].name,
          group: parameters[index].group,
          value: value,
          min: parameters[index].min,
          max: parameters[index].max,
          defaultValue: parameters[index].defaultValue,
          step: parameters[index].step,
          paramType: parameters[index].paramType,
          choices: parameters[index].choices,
        );
      }
    });

    onParametersUpdated();

    try {
      await plugin_api.setGeneratorParameter(
        generatorId: widget.generatorId,
        paramId: paramId,
        value: value,
      );
    } catch (e) {
      debugPrint('Error setting generator parameter: $e');
    }
  }

  @protected
  double getParameter(int paramId, double fallback) {
    try {
      return parameters.firstWhere((p) => p.id == paramId).value;
    } catch (e) {
      return fallback;
    }
  }

  @protected
  void onParametersUpdated() {}

  // ==========================================================================
  // DYNAMIC UI GENERATION
  // ==========================================================================

  /// Builds a fully automatic UI based on the Rust ParameterSpecs.
  /// Subclasses can call this inside `buildGeneratorBody` if they don't want a custom layout.
  Widget buildDynamicGeneratorBody(BuildContext context) {
    if (isLoading) {
      return const Center(child: CircularProgressIndicator());
    }

    if (errorMessage != null) {
      return Center(
        child: Text(
          errorMessage!,
          style: const TextStyle(color: Colors.redAccent),
        ),
      );
    }

    final groups = groupedParameters;

    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: groups.entries.map((entry) {
          return Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              _buildSectionHeader(entry.key), // The group name (e.g. "Master", "Oscillator 1")
              _buildSectionContainer(
                Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: entry.value.map((param) {
                    return Padding(
                      padding: const EdgeInsets.only(bottom: 16),
                      child: buildDynamicParameterWidget(param),
                    );
                  }).toList(),
                ),
              ),
              const SizedBox(height: 24),
            ],
          );
        }).toList(),
      ),
    );
  }

  /// Automatically routes the parameter to the correct universal widget based on its type.
  @protected
  Widget buildDynamicParameterWidget(plugin_api.UiPluginParameter param) {
    // Note: Adjust the enum names below based on how FRB generated them in Dart 
    // (e.g., ParameterValueType.float vs ParameterValueType.Float)
    switch (param.paramType) {
      case plugin_api.UiParameterType.float:
      case plugin_api.UiParameterType.int:
        return KarbeatFloatParam(
          paramId: param.id,
          name: param.name,
          value: param.value,
          min: param.min,
          max: param.max,
          defaultValue: param.defaultValue,
          step: param.step == 0.0 ? 0.01 : param.step,
          onChanged: (val) => setParameter(param.id, val),
        );
      case plugin_api.UiParameterType.choice:
        return KarbeatChoiceParam(
          paramId: param.id,
          name: param.name,
          value: param.value,
          choices: param.choices,
          defaultValue: param.defaultValue,
          onChanged: (val) => setParameter(param.id, val),
        );
      case plugin_api.UiParameterType.bool:
        return KarbeatBoolParam(
          paramId: param.id,
          name: param.name,
          value: param.value,
          defaultValue: param.defaultValue,
          onChanged: (val) => setParameter(param.id, val),
        );
    }
  }

  Widget _buildSectionHeader(String title) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Text(
        title,
        style: const TextStyle(
          color: Colors.cyanAccent,
          fontSize: 18,
          fontWeight: FontWeight.bold,
          letterSpacing: 1.2,
        ),
      ),
    );
  }

  Widget _buildSectionContainer(Widget child) {
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: const Color(0xFF16213E),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: Colors.grey.shade700.withAlpha(128)),
      ),
      child: child,
    );
  }

  /// Subclasses can override this to provide a custom UI layout, 
  /// but it defaults to the dynamic layout.
  Widget buildGeneratorBody(BuildContext context) {
    return buildDynamicGeneratorBody(context);
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: Colors.grey.shade900,
      appBar: AppBar(
        backgroundColor: const Color(0xFF16213E),
        elevation: 0,
        foregroundColor: Colors.white,
        title: Text(
          generatorName,
          style: const TextStyle(fontSize: 14, fontWeight: FontWeight.bold),
        ),
      ),
      body: buildGeneratorBody(context),
    );
  }
}