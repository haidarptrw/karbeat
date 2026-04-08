import 'package:flutter/material.dart';
import 'package:karbeat/features/audio_plugins/generators/abstract_generator_screen.dart';
import 'package:karbeat/src/rust/api/plugin.dart' as plugin_api;

class KarbeatzerScreen extends AbstractGeneratorScreen {
  const KarbeatzerScreen({super.key, required super.generatorId});

  @override
  KarbeatzerScreenState createState() {
    return KarbeatzerScreenState();
  }
}

class KarbeatzerScreenState
    extends AbstractGeneratorScreenState<KarbeatzerScreen> {
  // Waveform names for display
  static const List<String> _waveformNames = [
    'Sine',
    'Saw',
    'Square',
    'Triangle',
    'Noise',
  ];

  @override
  String get generatorName => 'Karbeatzer V2';

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

    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Master Section
          _buildSectionHeader('Master'),
          _buildMasterSection(),
          const SizedBox(height: 24),

          // Build filter
          _buildSectionHeader('Filter'),
          

          // Oscillator 1
          _buildSectionHeader('Oscillator 1'),
          _buildOscillatorSection(oscIndex: 0, baseParamId: 10),
          const SizedBox(height: 24),

          // Oscillator 2
          _buildSectionHeader('Oscillator 2'),
          _buildOscillatorSection(oscIndex: 1, baseParamId: 20),
          const SizedBox(height: 24),

          // Oscillator 3
          _buildSectionHeader('Oscillator 3'),
          _buildOscillatorSection(oscIndex: 2, baseParamId: 30),
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

  Widget _buildMasterSection() {
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: const Color(0xFF16213E),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: Colors.grey.shade700.withAlpha(128)),
      ),
      child: Row(
        children: [
          Expanded(
            child: _buildSliderParam(
              label: 'Drive',
              paramId: 8,
              min: 0.0,
              max: 1.0,
              defaultValue: 0.0,
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildOscillatorSection({
    required int oscIndex,
    required int baseParamId,
  }) {
    // Parameter IDs: base+0=waveform, base+1=detune, base+2=mix, base+3=pulse width
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
          // Waveform selector
          _buildWaveformSelector(paramId: baseParamId),
          const SizedBox(height: 16),

          // Detune slider
          _buildSliderParam(
            label: 'Detune',
            paramId: baseParamId + 1,
            min: -24.0,
            max: 24.0,
            defaultValue: 0.0,
            suffix: ' st',
          ),
          const SizedBox(height: 12),

          // Mix slider
          _buildSliderParam(
            label: 'Mix',
            paramId: baseParamId + 2,
            min: 0.0,
            max: 1.0,
            defaultValue: 1.0,
          ),
          const SizedBox(height: 12),

          // Pulse Width slider
          _buildSliderParam(
            label: 'Pulse Width',
            paramId: baseParamId + 3,
            min: 0.01,
            max: 0.99,
            defaultValue: 0.5,
          ),
        ],
      ),
    );
  }

  Widget _buildWaveformSelector({required int paramId}) {
    final currentWaveform = getParameter(paramId, 0).toInt();

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Text(
          'Waveform',
          style: TextStyle(color: Colors.grey, fontSize: 12),
        ),
        const SizedBox(height: 8),
        Wrap(
          spacing: 8,
          runSpacing: 8,
          children: List.generate(_waveformNames.length, (index) {
            final isSelected = currentWaveform == index;
            return GestureDetector(
              onTap: () => setParameter(paramId, index.toDouble()),
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
                  _waveformNames[index],
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

  Widget _buildSliderParam({
    required String label,
    required int paramId,
    required double min,
    required double max,
    required double defaultValue,
    String suffix = '',
  }) {
    final value = getParameter(paramId, defaultValue);
    final displayValue = value.toStringAsFixed(2);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            Text(
              label,
              style: const TextStyle(color: Colors.grey, fontSize: 12),
            ),
            Text(
              '$displayValue$suffix',
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
            value: value.clamp(min, max),
            min: min,
            max: max,
            onChanged: (newValue) => setParameter(paramId, newValue),
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
