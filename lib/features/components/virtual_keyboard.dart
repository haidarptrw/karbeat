import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

class VirtualKeyboard extends StatefulWidget {
  final int startOctave;
  final int octaveCount;
  final Function(int note) onNoteOn;
  final Function(int note) onNoteOff;
  final Set<int> activeNotes; // External active notes (e.g. from laptop keyboard)

  const VirtualKeyboard({
    super.key,
    this.startOctave = 4, // Start at C3 (MIDI 48)
    this.octaveCount = 2,
    required this.onNoteOn,
    required this.onNoteOff,
    required this.activeNotes,
  });

  @override
  State<VirtualKeyboard> createState() => _VirtualKeyboardState();
}

class _VirtualKeyboardState extends State<VirtualKeyboard> {
  // We calculate keys dynamically based on octave count
  late final int _startNote;
  late final int _totalKeys;

  @override
  void initState() {
    super.initState();
    _startNote = (widget.startOctave) * 12; // MIDI 48 for C3
    _totalKeys = widget.octaveCount * 12;
  }

  bool _isBlack(int note) {
    int n = note % 12;
    return n == 1 || n == 3 || n == 6 || n == 8 || n == 10;
  }

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        // Calculate white key width
        // 7 white keys per octave
        int whiteKeyCount = widget.octaveCount * 7; 
        double whiteKeyWidth = constraints.maxWidth / whiteKeyCount;

        List<Widget> whiteKeys = [];
        List<Widget> blackKeys = [];

        int whiteIndex = 0;

        for (int i = 0; i < _totalKeys; i++) {
          final note = _startNote + i;
          final isBlack = _isBlack(note);
          final isPressed = widget.activeNotes.contains(note);

          if (!isBlack) {
            // WHITE KEY
            whiteKeys.add(
              _PianoKey(
                note: note,
                isBlack: false,
                width: whiteKeyWidth,
                height: constraints.maxHeight,
                isPressed: isPressed,
                onNoteOn: widget.onNoteOn,
                onNoteOff: widget.onNoteOff,
              ),
            );
            whiteIndex++;
          } else {
            // BLACK KEY
            // Positioned relative to the previous white key
            // Black keys are usually centered on the line between white keys
            double left = (whiteIndex * whiteKeyWidth) - (whiteKeyWidth * 0.35); // Slight offset
            
            blackKeys.add(
              Positioned(
                left: left,
                width: whiteKeyWidth * 0.7,
                height: constraints.maxHeight * 0.6,
                child: _PianoKey(
                  note: note,
                  isBlack: true,
                  width: whiteKeyWidth * 0.7,
                  height: constraints.maxHeight * 0.6,
                  isPressed: isPressed,
                  onNoteOn: widget.onNoteOn,
                  onNoteOff: widget.onNoteOff,
                ),
              ),
            );
          }
        }

        return Stack(
          children: [
            Row(children: whiteKeys),
            ...blackKeys,
          ],
        );
      },
    );
  }
}

class _PianoKey extends StatefulWidget {
  final int note;
  final bool isBlack;
  final bool isPressed;
  final double width;
  final double height;
  final Function(int) onNoteOn;
  final Function(int) onNoteOff;

  const _PianoKey({
    required this.note,
    required this.isBlack,
    required this.isPressed,
    required this.width,
    required this.height,
    required this.onNoteOn,
    required this.onNoteOff,
  });

  @override
  State<_PianoKey> createState() => _PianoKeyState();
}

class _PianoKeyState extends State<_PianoKey> {
  bool _touchActive = false;

  @override
  Widget build(BuildContext context) {
    // Combine external press (keyboard) and internal touch
    final isActive = widget.isPressed || _touchActive;

    return Listener(
      onPointerDown: (_) {
        setState(() => _touchActive = true);
        widget.onNoteOn(widget.note);
      },
      onPointerUp: (_) {
        setState(() => _touchActive = false);
        widget.onNoteOff(widget.note);
      },
      onPointerCancel: (_) {
        setState(() => _touchActive = false);
        widget.onNoteOff(widget.note);
      },
      child: Container(
        width: widget.width,
        height: widget.height,
        decoration: BoxDecoration(
          color: isActive
              ? Colors.cyanAccent
              : (widget.isBlack ? Colors.black : Colors.white),
          border: Border.all(color: Colors.black, width: 1),
          borderRadius: BorderRadius.only(
            bottomLeft: const Radius.circular(4),
            bottomRight: const Radius.circular(4),
          ),
        ),
        child: Align(
          alignment: Alignment.bottomCenter,
          child: Padding(
            padding: const EdgeInsets.only(bottom: 4.0),
            child: Text(
              _getNoteLabel(widget.note),
              style: TextStyle(
                color: isActive 
                  ? Colors.black 
                  : (widget.isBlack ? Colors.white54 : Colors.black54),
                fontSize: 8,
              ),
            ),
          ),
        ),
      ),
    );
  }

  String _getNoteLabel(int note) {
    const names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    // Only show C notes label to reduce clutter
    if (note % 12 == 0) return "C${(note / 12).floor()}";
    return "";
  }
}