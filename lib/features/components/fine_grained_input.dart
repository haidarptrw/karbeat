import 'package:flutter/material.dart';
import 'package:karbeat/features/components/context_menu.dart';

/// A widget wrapper class that enables a fine-grained input setter 
/// (i.e., setting a specific value from a slider type input). When we
/// trigger the activate button (long press), it will open a dialog that allows us to
/// set the value directly using text input with a button to increment or decrement.
class FineGrainedInputWrapper<T extends num> extends StatelessWidget {
  final Widget child;
  final T value;
  final T step;
  final T min;
  final T max;
  
  final ValueChanged<T> onChanged;

  const FineGrainedInputWrapper({
    super.key,
    required this.child,
    required this.value,
    required this.onChanged,
    required this.step,
    required this.min,
    required this.max
  });

  @override
  Widget build(BuildContext context) {
    // We use a GestureDetector to catch long presses (mobile) 
    // or right-clicks (desktop) without breaking normal Slider drag behavior.
    return GestureDetector(
      onLongPress: () => _showFineGrainedDialog(context),
      onSecondaryTap: () => _showFineGrainedDialog(context),
      child: child,
    );
  }

  void _showFineGrainedDialog(BuildContext context) {
    showDialog(
      context: context,
      builder: (context) {
        return _FineGrainedDialog<T>(
          initialValue: value,
          step: step,
          min: min,
          max: max,
          onChanged: onChanged,
        );
      },
    );
  }
}

class _FineGrainedDialog<T extends num> extends StatefulWidget {
  final T initialValue;
  final T step;
  final T min;
  final T max;
  final ValueChanged<T> onChanged;

  const _FineGrainedDialog({
    required this.initialValue,
    required this.step,
    required this.min,
    required this.max,
    required this.onChanged,
  });

  @override
  State<_FineGrainedDialog<T>> createState() => _FineGrainedDialogState<T>();
}

class _FineGrainedDialogState<T extends num> extends State<_FineGrainedDialog<T>> {
  late TextEditingController _controller;
  late T _currentValue;

  @override
  void initState() {
    super.initState();
    _currentValue = widget.initialValue;
    _controller = TextEditingController(text: _formatValue(_currentValue));
  }
  
  String _formatValue(T val) {
    // Prevents endless decimal tails for doubles
    return val is double ? val.toStringAsFixed(2) : val.toString();
  }

  void _updateValue(T newValue) {
    setState(() {
      _currentValue = newValue.clamp(widget.min, widget.max) as T;
      _controller.text = _formatValue(_currentValue);
    });
  }

  void _increment() {
    _updateValue((_currentValue + widget.step) as T);
  }

  void _decrement() {
    _updateValue((_currentValue - widget.step) as T);
  }

  void _submit() {
    final parsed = num.tryParse(_controller.text);
    if (parsed != null) {
      // Cast back to the correct generic type before returning
      T finalValue = (T == int ? parsed.toInt() : parsed.toDouble()) as T;
      finalValue = finalValue.clamp(widget.min, widget.max) as T;
      widget.onChanged(finalValue);
    }
    Navigator.of(context).pop();
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Exact Value'),
      content: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          IconButton(
            icon: const Icon(Icons.remove),
            onPressed: _decrement,
          ),
          SizedBox(
            width: 80,
            child: TextField(
              controller: _controller,
              keyboardType: const TextInputType.numberWithOptions(decimal: true, signed: true),
              textAlign: TextAlign.center,
              onSubmitted: (_) => _submit(),
              // Select all text automatically when tapped for easy overwriting
              onTap: () => _controller.selection = TextSelection(
                baseOffset: 0,
                extentOffset: _controller.text.length,
              ),
            ),
          ),
          IconButton(
            icon: const Icon(Icons.add),
            onPressed: _increment,
          ),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        ElevatedButton(
          onPressed: _submit,
          child: const Text('OK'),
        ),
      ],
    );
  }
}

/// Opens the fine-grained input dialog programmatically.
Future<void> showFineGrainedDialog<T extends num>({
  required BuildContext context,
  required T initialValue,
  required T step,
  required T min,
  required T max,
  required ValueChanged<T> onChanged,
}) {
  return showDialog(
    context: context,
    builder: (context) {
      return _FineGrainedDialog<T>(
        initialValue: initialValue,
        step: step,
        min: min,
        max: max,
        onChanged: onChanged,
      );
    },
  );
}

/// A master wrapper for DAW parameters (knobs, sliders).
/// It captures right-click/long-press to show a standardized Context Menu.
/// Features:
/// 1. Type in exact value (opens FineGrainedDialog)
/// 2. Reset to default
/// 3. Create automation clip (optional)
class ParameterInteractionWrapper<T extends num> extends StatelessWidget {
  final Widget child;
  final String parameterName;
  final T value;
  final T defaultValue;
  final T min;
  final T max;
  final T step;
  final ValueChanged<T> onChanged;
  final VoidCallback? onAddAutomation;

  const ParameterInteractionWrapper({
    super.key,
    required this.child,
    required this.parameterName,
    required this.value,
    required this.defaultValue,
    required this.min,
    required this.max,
    required this.step,
    required this.onChanged,
    this.onAddAutomation,
  });

  @override
  Widget build(BuildContext context) {
    return ContextMenuWrapper(
      title: parameterName,
      actions: [
        KarbeatContextAction(
          title: "Type in value...",
          icon: Icons.keyboard,
          onTap: () {
            // Because the ContextMenuWrapper automatically pops the context menu 
            // dialog before calling onTap, we can safely open the new dialog here.
            showFineGrainedDialog<T>(
              context: context,
              initialValue: value,
              step: step,
              min: min,
              max: max,
              onChanged: onChanged,
            );
          },
        ),
        KarbeatContextAction(
          title: "Reset to default",
          icon: Icons.refresh,
          onTap: () {
            onChanged(defaultValue);
          },
        ),
        if (onAddAutomation != null)
          KarbeatContextAction(
            title: "Create automation clip",
            icon: Icons.show_chart,
            onTap: onAddAutomation!,
          ),
      ],
      child: child,
    );
  }
}