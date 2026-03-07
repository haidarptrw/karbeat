import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:karbeat/src/rust/api/plugin.dart' as plugin_api;

/// Abstract base class for effect plugin screens.
///
/// Provides default implementations for:
/// - Parameter polling (from audio thread feedback)
/// - Loading parameter specs
/// - Setting parameters (optimistic UI + backend)
/// - Standard Scaffold/AppBar layout
///
/// Subclasses must implement [buildEffectBody] to define the custom effect UI.
abstract class AbstractEffectScreen extends ConsumerStatefulWidget {
  final int trackId;
  final int effectIdx;

  const AbstractEffectScreen({
    super.key,
    required this.trackId,
    required this.effectIdx,
  });
}

abstract class AbstractEffectScreenState<T extends AbstractEffectScreen>
    extends ConsumerState<T> {
  List<plugin_api.UiPluginParameter> parameters = [];
  bool isLoading = true;
  String? errorMessage;

  /// Timer for polling parameter feedback from audio thread
  Timer? _parameterPollTimer;

  /// Display name for the effect (shown in AppBar).
  /// Override this in subclasses to customize.
  String get effectName => 'Effect';

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

  /// Start polling for parameter feedback from the audio thread.
  /// Sends an initial query, then polls every 50ms for updates.
  @protected
  void startParameterPolling() async {
    // Request initial parameter snapshot from audio thread
    await plugin_api.queryEffectParameters(
      trackId: widget.trackId,
      effectId: widget.effectIdx,
    );

    // Poll every 50ms for smooth UI updates during automation
    _parameterPollTimer = Timer.periodic(
      const Duration(milliseconds: 50),
      (_) => _pollParameterFeedback(),
    );
  }

  /// Poll for parameter feedback from the audio thread and update UI.
  void _pollParameterFeedback() async {
    if (!mounted) return;

    try {
      final snapshots = await plugin_api.pollEffectParameterFeedback();
      if (snapshots.isEmpty) return;

      // Sync to stored parameters (for persistence)
      await plugin_api.syncEffectParametersFromAudio(snapshots: snapshots);

      // Update local UI state
      setState(() {
        for (final snapshot in snapshots) {
          // Only process snapshots for this effect
          if (snapshot.effectId != widget.effectIdx) continue;

          for (final paramValue in snapshot.parameters) {
            final index = parameters.indexWhere(
              (p) => p.id == paramValue.paramId,
            );
            if (index != -1) {
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
            }
          }
        }
      });
    } catch (e) {
      debugPrint('Error polling effect parameter feedback: $e');
    }
  }

  /// Load parameter specs from the backend.
  /// Override in subclasses if custom initialization is needed.
  @protected
  Future<void> loadParameterSpecs() async {
    try {
      final specs = await plugin_api.getEffectParameterSpecs(
        trackId: widget.trackId,
        effectId: widget.effectIdx,
      );
      setState(() {
        parameters = specs;
        isLoading = false;
      });
    } catch (e) {
      setState(() {
        errorMessage = 'Failed to load effect parameters: $e';
        isLoading = false;
      });
    }
  }

  /// Set a parameter value with optimistic local update + backend sync.
  @protected
  Future<void> setParameter(int paramId, double value) async {
    // Update local state immediately for smooth UI
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

    // Send to backend
    try {
      await plugin_api.setEffectParameter(
        trackId: widget.trackId,
        effectId: widget.effectIdx,
        paramId: paramId,
        value: value,
      );
    } catch (e) {
      debugPrint('Error setting effect parameter: $e');
    }
  }

  /// Subclasses must implement this to define the custom effect UI.
  Widget buildEffectBody(BuildContext context);

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: Colors.grey.shade900,
      appBar: AppBar(
        backgroundColor: const Color(0xFF16213E),
        elevation: 0,
        foregroundColor: Colors.white,
        title: Text(
          effectName,
          style: const TextStyle(fontSize: 14, fontWeight: FontWeight.bold),
        ),
      ),
      body: buildEffectBody(context),
    );
  }
}
