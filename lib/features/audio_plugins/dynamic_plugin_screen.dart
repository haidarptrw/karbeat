import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:karbeat/features/audio_plugins/generators/abstract_generator_screen.dart';
import 'package:karbeat/features/components/scrollable_virtual_keyboard.dart';
import 'package:karbeat/src/rust/api/audio.dart' as audio_api;
import 'package:karbeat/src/rust/api/plugin.dart' as plugin_api;
import 'package:karbeat/state/app_state.dart';

/// Generic Dynamic Plugin Screen that generates UI from plugin parameter specs.
/// This acts as a fallback for generators that do not have custom UI.
class DynamicPluginScreen extends AbstractGeneratorScreen {
  final String _generatorName;

  const DynamicPluginScreen({
    super.key,
    required super.generatorId,
    required String generatorName,
  }) : _generatorName = generatorName;

  @override
  ConsumerState<DynamicPluginScreen> createState() =>
      _DynamicPluginScreenState();
}

class _DynamicPluginScreenState
    extends AbstractGeneratorScreenState<DynamicPluginScreen> {
  // Track active notes for keyboard visualization
  final Set<int> _activeNotes = {};

  @override
  String get generatorName => widget._generatorName;

  @override
  Future<void> setParameter(int paramId, double value) async {
    await super.setParameter(paramId, value);
    // Sync generator list so Flutter state matches backend
    if (mounted) {
      await ref
          .read(karbeatStateProvider)
          .syncGenerator(generatorId: widget.generatorId);
    }
  }

  /// Group parameters by their group field
  Map<String, List<plugin_api.UiPluginParameter>> _groupedParameters() {
    final grouped = <String, List<plugin_api.UiPluginParameter>>{};
    for (final param in parameters) {
      grouped.putIfAbsent(param.group, () => []).add(param);
    }
    return grouped;
  }

  @override
  Widget buildGeneratorBody(BuildContext context) {
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

    final grouped = _groupedParameters();

    return Column(
      children: [
        Expanded(
          child: SingleChildScrollView(
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
          ),
        ),
        // Scrollable MIDI Keyboard
        ScrollableVirtualKeyboard(
          height: 120,
          onNoteOn: _handleNoteOn,
          onNoteOff: _handleNoteOff,
          activeNotes: _activeNotes,
          initialCenterNote: 72,
        ),
      ],
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
            onChanged: (newValue) => setParameter(param.id, newValue),
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
          onChanged: (value) => setParameter(param.id, value ? 1.0 : 0.0),
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
              onTap: () => setParameter(param.id, index.toDouble()),
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
