import 'dart:async';

import 'package:flutter/material.dart';
import 'package:karbeat/features/components/scrollable_virtual_keyboard.dart';
import 'package:karbeat/src/rust/api/audio.dart' as audio_api;
import 'package:karbeat/src/rust/api/plugin.dart' as plugin_api;
import 'package:karbeat/state/app_state.dart';
import 'package:provider/provider.dart';

/// Generic Dynamic Plugin Screen that generates UI from plugin parameter specs.
/// This replaces manual plugin screens like KarbeatzerScreen.
class DynamicPluginScreen extends StatefulWidget {
  final int generatorId;
  final String generatorName;

  const DynamicPluginScreen({
    super.key,
    required this.generatorId,
    required this.generatorName,
  });

  @override
  State<DynamicPluginScreen> createState() => _DynamicPluginScreenState();
}

class _DynamicPluginScreenState extends State<DynamicPluginScreen> {
  List<plugin_api.UiPluginParameter> _parameters = [];
  bool _isLoading = true;
  String? _errorMessage;

  // Track active notes for keyboard visualization
  final Set<int> _activeNotes = {};

  // Timer for polling parameter feedback from audio thread
  Timer? _parameterPollTimer;

  @override
  void initState() {
    super.initState();
    _loadParameterSpecs();
    _startParameterPolling();
  }

  @override
  void dispose() {
    _parameterPollTimer?.cancel();
    super.dispose();
  }

  /// Start polling for parameter feedback from the audio thread.
  /// This enables live UI updates when parameters change via automation.
  void _startParameterPolling() async {
    // Request initial parameter snapshot from audio thread
    await plugin_api.queryGeneratorParameters(generatorId: widget.generatorId);

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
      // 1. Poll for pending feedback from audio thread
      final snapshots = await plugin_api.pollParameterFeedback();

      if (snapshots.isEmpty) return;

      // 2. Sync to stored parameters (for persistence)
      await plugin_api.syncParametersFromAudio(snapshots: snapshots);

      // 3. Update local UI state
      setState(() {
        for (final snapshot in snapshots) {
          // Only process snapshots for this generator
          if (snapshot.generatorId != widget.generatorId) continue;

          for (final paramValue in snapshot.parameters) {
            final index = _parameters.indexWhere(
              (p) => p.id == paramValue.paramId,
            );
            if (index != -1) {
              // Update the parameter value in local state
              _parameters[index] = plugin_api.UiPluginParameter(
                id: _parameters[index].id,
                name: _parameters[index].name,
                group: _parameters[index].group,
                value: paramValue.value, // Updated value from audio
                min: _parameters[index].min,
                max: _parameters[index].max,
                defaultValue: _parameters[index].defaultValue,
                step: _parameters[index].step,
                paramType: _parameters[index].paramType,
                choices: _parameters[index].choices,
              );
            }
          }
        }
      });
    } catch (e) {
      debugPrint('Error polling parameter feedback: $e');
    }
  }

  Future<void> _loadParameterSpecs() async {
    try {
      final specs = await plugin_api.getGeneratorParameterSpecs(
        generatorId: widget.generatorId,
      );
      setState(() {
        _parameters = specs;
        _isLoading = false;
      });
    } catch (e) {
      setState(() {
        _errorMessage = 'Failed to load parameters: $e';
        _isLoading = false;
      });
    }
  }

  Future<void> _setParameter(int paramId, double value) async {
    // Update local state immediately for smooth UI
    setState(() {
      final index = _parameters.indexWhere((p) => p.id == paramId);
      if (index != -1) {
        _parameters[index] = plugin_api.UiPluginParameter(
          id: _parameters[index].id,
          name: _parameters[index].name,
          group: _parameters[index].group,
          value: value,
          min: _parameters[index].min,
          max: _parameters[index].max,
          defaultValue: _parameters[index].defaultValue,
          step: _parameters[index].step,
          paramType: _parameters[index].paramType,
          choices: _parameters[index].choices,
        );
      }
    });

    // Send to backend
    try {
      await plugin_api.setGeneratorParameter(
        generatorId: widget.generatorId,
        paramId: paramId,
        value: value,
      );
      // Sync generator list so Flutter state matches backend
      if (mounted) {
        await context.read<KarbeatState>().syncGenerator(
          generatorId: widget.generatorId,
        );
      }
    } catch (e) {
      debugPrint('Error setting parameter: $e');
    }
  }

  /// Group parameters by their group field
  Map<String, List<plugin_api.UiPluginParameter>> _groupedParameters() {
    final grouped = <String, List<plugin_api.UiPluginParameter>>{};
    for (final param in _parameters) {
      grouped.putIfAbsent(param.group, () => []).add(param);
    }
    return grouped;
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: Colors.grey.shade900,
      appBar: AppBar(
        title: Text(widget.generatorName),
        backgroundColor: const Color(0xFF1A1A2E),
        elevation: 0,
        foregroundColor: Colors.white,
      ),
      body: Column(
        children: [
          Expanded(child: _buildBody()),
          // Scrollable MIDI Keyboard
          ScrollableVirtualKeyboard(
            height: 120,
            onNoteOn: _handleNoteOn,
            onNoteOff: _handleNoteOff,
            activeNotes: _activeNotes,
            initialCenterNote: 60,
          ),
        ],
      ),
    );
  }

  void _handleNoteOn(int note) async {
    setState(() => _activeNotes.add(note));
    try {
      await audio_api.playPreviewNoteGenerator(
        generatorId: widget.generatorId,
        noteKey: note,
        velocity: 100,
        isOn: true,
      );
    } catch (e) {
      debugPrint('Error playing note on: $e');
    }
  }

  void _handleNoteOff(int note) async {
    setState(() => _activeNotes.remove(note));
    try {
      await audio_api.playPreviewNoteGenerator(
        generatorId: widget.generatorId,
        noteKey: note,
        velocity: 100,
        isOn: false,
      );
    } catch (e) {
      debugPrint('Error playing note off: $e');
    }
  }

  Widget _buildBody() {
    if (_isLoading) {
      return const Center(child: CircularProgressIndicator());
    }

    if (_errorMessage != null) {
      return Center(
        child: Text(
          _errorMessage!,
          style: const TextStyle(color: Colors.redAccent),
        ),
      );
    }

    final grouped = _groupedParameters();

    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          for (final entry in grouped.entries) ...[
            _buildSectionHeader(entry.key),
            _buildParameterSection(entry.value),
            const SizedBox(height: 24),
          ],
        ],
      ),
    );
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

  Widget _buildParameterSection(List<plugin_api.UiPluginParameter> params) {
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: const Color(0xFF16213E),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: Colors.grey.shade700.withAlpha(128)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          for (int i = 0; i < params.length; i++) ...[
            _buildParameterWidget(params[i]),
            if (i < params.length - 1) const SizedBox(height: 12),
          ],
        ],
      ),
    );
  }

  Widget _buildParameterWidget(plugin_api.UiPluginParameter param) {
    switch (param.paramType) {
      case plugin_api.UiParameterType.bool:
        return _buildBoolParameter(param);
      case plugin_api.UiParameterType.choice:
        return _buildChoiceParameter(param);
      case plugin_api.UiParameterType.float:
        return _buildSliderParameter(param);
      case plugin_api.UiParameterType.int:
        return _buildSliderParameter(param);
    }
  }

  Widget _buildSliderParameter(plugin_api.UiPluginParameter param) {
    final value = param.value.clamp(param.min, param.max);
    final displayValue = param.step >= 1.0
        ? value.toInt().toString()
        : value.toStringAsFixed(2);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            Text(
              param.name,
              style: const TextStyle(color: Colors.grey, fontSize: 12),
            ),
            Text(
              displayValue,
              style: const TextStyle(color: Colors.white70, fontSize: 12),
            ),
          ],
        ),
        const SizedBox(height: 4),
        SliderTheme(
          data: SliderTheme.of(context).copyWith(
            activeTrackColor: Colors.cyanAccent,
            inactiveTrackColor: Colors.grey.shade700,
            thumbColor: Colors.cyanAccent,
            overlayColor: Colors.cyanAccent.withAlpha(51),
            trackHeight: 4,
          ),
          child: Slider(
            value: value,
            min: param.min,
            max: param.max,
            onChanged: (newValue) => _setParameter(param.id, newValue),
          ),
        ),
      ],
    );
  }

  Widget _buildBoolParameter(plugin_api.UiPluginParameter param) {
    final isOn = param.value >= 0.5;

    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Text(
          param.name,
          style: const TextStyle(color: Colors.grey, fontSize: 14),
        ),
        Switch(
          value: isOn,
          activeThumbColor: Colors.cyanAccent,
          onChanged: (value) => _setParameter(param.id, value ? 1.0 : 0.0),
        ),
      ],
    );
  }

  Widget _buildChoiceParameter(plugin_api.UiPluginParameter param) {
    final currentChoice = param.value.toInt().clamp(
      0,
      param.choices.length - 1,
    );

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          param.name,
          style: const TextStyle(color: Colors.grey, fontSize: 12),
        ),
        const SizedBox(height: 8),
        Wrap(
          spacing: 8,
          runSpacing: 8,
          children: List.generate(param.choices.length, (index) {
            final isSelected = currentChoice == index;
            return GestureDetector(
              onTap: () => _setParameter(param.id, index.toDouble()),
              child: AnimatedContainer(
                duration: const Duration(milliseconds: 150),
                padding: const EdgeInsets.symmetric(
                  horizontal: 16,
                  vertical: 10,
                ),
                decoration: BoxDecoration(
                  color: isSelected
                      ? Colors.cyanAccent.withAlpha(51)
                      : Colors.grey.shade800,
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(
                    color: isSelected
                        ? Colors.cyanAccent
                        : Colors.grey.shade700,
                    width: isSelected ? 2 : 1,
                  ),
                ),
                child: Text(
                  param.choices[index],
                  style: TextStyle(
                    color: isSelected ? Colors.cyanAccent : Colors.white70,
                    fontWeight: isSelected
                        ? FontWeight.bold
                        : FontWeight.normal,
                  ),
                ),
              ),
            );
          }),
        ),
      ],
    );
  }
}
