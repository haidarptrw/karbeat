import 'package:flutter/material.dart';
// Make sure to import your ParameterInteractionWrapper here!
import 'package:karbeat/features/components/fine_grained_input.dart'; 

// ============================================================================
// FLOAT PARAMETER (Slider)
// ============================================================================
class KarbeatFloatParam extends StatelessWidget {
  final int paramId;
  final String name;
  final double value;
  final double min;
  final double max;
  final double defaultValue;
  final double step;
  final String suffix;
  final ValueChanged<double> onChanged;

  const KarbeatFloatParam({
    super.key,
    required this.paramId,
    required this.name,
    required this.value,
    required this.min,
    required this.max,
    required this.defaultValue,
    this.step = 0.01,
    this.suffix = '',
    required this.onChanged,
  });

  @override
  Widget build(BuildContext context) {
    final displayValue = value.toStringAsFixed(2);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            Text(
              name,
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
          child: ParameterInteractionWrapper<double>(
            parameterName: name,
            value: value,
            defaultValue: defaultValue,
            min: min,
            max: max,
            step: step,
            onChanged: onChanged,
            onAddAutomation: () {
              // TODO: Wire up automation lane creation using paramId
              debugPrint("Create automation for $name ($paramId)");
            },
            child: Slider(
              value: value.clamp(min, max),
              min: min,
              max: max,
              onChanged: onChanged,
            ),
          ),
        ),
      ],
    );
  }
}

// ============================================================================
// CHOICE PARAMETER (Chips)
// ============================================================================
class KarbeatChoiceParam extends StatelessWidget {
  final int paramId;
  final String name;
  final double value;
  final List<String> choices;
  final double defaultValue;
  final ValueChanged<double> onChanged;

  const KarbeatChoiceParam({
    super.key,
    required this.paramId,
    required this.name,
    required this.value,
    required this.choices,
    required this.defaultValue,
    required this.onChanged,
  });

  @override
  Widget build(BuildContext context) {
    final currentChoice = value.toInt().clamp(0, choices.length - 1);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          name,
          style: const TextStyle(color: Colors.grey, fontSize: 12),
        ),
        const SizedBox(height: 8),
        ParameterInteractionWrapper<double>(
          parameterName: name,
          value: value,
          defaultValue: defaultValue,
          min: 0,
          max: (choices.length - 1).toDouble(),
          step: 1.0,
          onChanged: onChanged,
          onAddAutomation: () {
            debugPrint("Create automation for $name ($paramId)");
          },
          child: Wrap(
            spacing: 8,
            runSpacing: 8,
            children: List.generate(choices.length, (index) {
              final isSelected = currentChoice == index;
              return GestureDetector(
                onTap: () => onChanged(index.toDouble()),
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
                    choices[index],
                    style: TextStyle(
                      color: isSelected ? Colors.cyanAccent : Colors.white70,
                      fontWeight: isSelected ? FontWeight.bold : FontWeight.normal,
                    ),
                  ),
                ),
              );
            }),
          ),
        ),
      ],
    );
  }
}

// ============================================================================
// BOOL PARAMETER (Switch)
// ============================================================================
class KarbeatBoolParam extends StatelessWidget {
  final int paramId;
  final String name;
  final double value;
  final double defaultValue;
  final ValueChanged<double> onChanged;

  const KarbeatBoolParam({
    super.key,
    required this.paramId,
    required this.name,
    required this.value,
    required this.defaultValue,
    required this.onChanged,
  });

  @override
  Widget build(BuildContext context) {
    final isOn = value >= 0.5;

    return ParameterInteractionWrapper<double>(
      parameterName: name,
      value: value,
      defaultValue: defaultValue,
      min: 0.0,
      max: 1.0,
      step: 1.0,
      onChanged: onChanged,
      onAddAutomation: () {
        debugPrint("Create automation for $name ($paramId)");
      },
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Text(
            name,
            style: const TextStyle(color: Colors.grey, fontSize: 14),
          ),
          Switch(
            value: isOn,
            activeThumbColor: Colors.cyanAccent,
            onChanged: (val) => onChanged(val ? 1.0 : 0.0),
          ),
        ],
      ),
    );
  }
}