import 'package:flutter/material.dart';
import 'package:karbeat/src/rust/api/plugin.dart' as plugin_api;
import 'package:karbeat/state/app_state.dart';
import 'package:provider/provider.dart';

/// UI Screen for the Karbeatzer V2 Synthesizer
class KarbeatzerScreen extends StatefulWidget {
  final int generatorId;
  final String generatorName;

  const KarbeatzerScreen({
    super.key,
    required this.generatorId,
    required this.generatorName,
  });

  @override
  State<KarbeatzerScreen> createState() => _KarbeatzerScreenState();
}

class _KarbeatzerScreenState extends State<KarbeatzerScreen> {
  // Local state for parameters (for smooth UI updates)
  Map<int, double> _parameters = {};
  bool _isLoading = true;

  // Waveform names for display
  static const List<String> _waveformNames = [
    'Sine',
    'Saw',
    'Square',
    'Triangle',
    'Noise',
  ];

  @override
  void initState() {
    super.initState();
    _loadParameters();
  }

  Future<void> _loadParameters() async {
    // Get parameters from state
    final state = context.read<KarbeatState>();
    final generator = state.generators[widget.generatorId];

    if (generator != null) {
      setState(() {
        _parameters = Map<int, double>.from(generator.parameters);
        _isLoading = false;
      });
    } else {
      setState(() => _isLoading = false);
    }
  }

  Future<void> _setParameter(int paramId, double value) async {
    // Update local state immediately for smooth UI
    setState(() {
      _parameters[paramId] = value;
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
        await context.read<KarbeatState>().syncGeneratorList();
      }
    } catch (e) {
      debugPrint('Error setting parameter: $e');
    }
  }

  double _getParameter(int paramId, {double defaultValue = 0.0}) {
    return _parameters[paramId] ?? defaultValue;
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
      body: _isLoading
          ? const Center(child: CircularProgressIndicator())
          : SingleChildScrollView(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  // Master Section
                  _buildSectionHeader('Master'),
                  _buildMasterSection(),
                  const SizedBox(height: 24),

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
    final currentWaveform = _getParameter(paramId, defaultValue: 0).toInt();

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
              onTap: () => _setParameter(paramId, index.toDouble()),
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
    final value = _getParameter(paramId, defaultValue: defaultValue);
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
            onChanged: (newValue) => _setParameter(paramId, newValue),
          ),
        ),
      ],
    );
  }
}
