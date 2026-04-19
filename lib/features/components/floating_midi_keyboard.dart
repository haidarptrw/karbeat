import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:karbeat/models/piano_key.dart';
import 'package:karbeat/src/rust/api/project.dart';
import 'package:karbeat/state/app_state.dart';
import 'package:karbeat/src/rust/api/audio.dart' as audio_api;
import 'package:karbeat/utils/formatter.dart';

class FloatingMidiKeyboard extends ConsumerStatefulWidget {
  const FloatingMidiKeyboard({super.key});

  @override
  ConsumerState<FloatingMidiKeyboard> createState() => _FloatingMidiKeyboardState();
}

class _FloatingMidiKeyboardState extends ConsumerState<FloatingMidiKeyboard> {
  double _x = 100;
  double _y = 100;

  int _baseKey = 48; // C3
  int _keyRange = 15; // 15 semitones
  int? _selectedGeneratorId;
  final Set<int> _activeNotes = {};

  String _getGeneratorName(UiGeneratorInstance instance) {
    return instance.instanceType.when(
      plugin: (p) => p.name,
      sampler: (assetId, rootNote) => "Sampler",
    );
  }

  @override
  Widget build(BuildContext context) {
    final generators = ref.watch(karbeatStateProvider.select((s) => s.generators));

    // Ensure selected generator is still valid, else pick first available
    if (_selectedGeneratorId != null && !generators.containsKey(_selectedGeneratorId)) {
      _selectedGeneratorId = null;
    }
    if (_selectedGeneratorId == null && generators.isNotEmpty) {
      _selectedGeneratorId = generators.keys.first;
    }

    return Positioned(
      left: _x,
      top: _y,
      child: Focus(
        autofocus: true,
        onKeyEvent: (node, event) {
          if (event is KeyDownEvent) {
            final baseNote = keyMap[event.physicalKey];
            if (baseNote != null) {
              final note = baseNote + (_baseKey - 48);
              if (!_activeNotes.contains(note)) {
                _handleNoteOn(note);
                return KeyEventResult.handled;
              }
            }
          } else if (event is KeyUpEvent) {
            final baseNote = keyMap[event.physicalKey];
            if (baseNote != null) {
              final note = baseNote + (_baseKey - 48);
              _handleNoteOff(note);
              return KeyEventResult.handled;
            }
          }
          return KeyEventResult.ignored;
        },
        child: Material(
          color: Colors.transparent,
          elevation: 12,
          borderRadius: BorderRadius.circular(8),
        child: Container(
          width: 520,
          decoration: BoxDecoration(
            color: const Color(0xFF1E1E1E), // Hardware synth look
            borderRadius: BorderRadius.circular(8),
            border: Border.all(color: Colors.grey.shade700),
            boxShadow: const [
              BoxShadow(
                color: Colors.black54,
                blurRadius: 10,
                offset: Offset(0, 5),
              ),
            ],
          ),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              // Draggable Top Bar
              GestureDetector(
                onPanUpdate: (details) {
                  setState(() {
                    _x += details.delta.dx;
                    _y += details.delta.dy;
                  });
                },
                child: Container(
                  padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                  decoration: BoxDecoration(
                    color: Colors.grey.shade900,
                    borderRadius: const BorderRadius.vertical(top: Radius.circular(8)),
                    border: Border(bottom: BorderSide(color: Colors.grey.shade800)),
                  ),
                  child: Row(
                    children: [
                      const Icon(Icons.drag_indicator, color: Colors.grey, size: 16),
                      const SizedBox(width: 8),
                      const Text(
                        "MIDI CONTROLLER",
                        style: TextStyle(
                          color: Colors.white70,
                          fontSize: 12,
                          fontWeight: FontWeight.bold,
                          letterSpacing: 2,
                        ),
                      ),
                      const Spacer(),
                      InkWell(
                        onTap: () => ref.read(karbeatStateProvider).toggleFloatingMidiKeyboard(),
                        child: const Icon(Icons.close, color: Colors.grey, size: 16),
                      ),
                    ],
                  ),
                ),
              ),
              
              // Synth Control Panel
              Container(
                padding: const EdgeInsets.all(12),
                color: const Color(0xFF232323),
                child: Row(
                  children: [
                    _buildControlKnob(
                      "BASE KEY", 
                      numToMidiKey(_baseKey), 
                      () {
                         setState(() { _baseKey = (_baseKey - 1).clamp(0, 127 - _keyRange); });
                      },
                      () {
                         setState(() { _baseKey = (_baseKey + 1).clamp(0, 127 - _keyRange); });
                      }
                    ),
                    const SizedBox(width: 20),
                    _buildControlKnob(
                      "RANGE", 
                      "+$_keyRange", 
                      () {
                         setState(() { _keyRange = (_keyRange - 1).clamp(1, 19); });
                      },
                      () {
                         setState(() { _keyRange = (_keyRange + 1).clamp(1, 19); });
                      }
                    ),
                    const Spacer(),
                    Container(
                      height: 36,
                      padding: const EdgeInsets.symmetric(horizontal: 12),
                      decoration: BoxDecoration(
                        color: Colors.black,
                        border: Border.all(color: Colors.red.shade900),
                        borderRadius: BorderRadius.circular(4),
                      ),
                      child: DropdownButtonHideUnderline(
                        child: DropdownButton<int>(
                          value: _selectedGeneratorId,
                          hint: const Text("Select Synth", style: TextStyle(color: Colors.redAccent, fontSize: 12)),
                          dropdownColor: Colors.black,
                          icon: const Icon(Icons.arrow_drop_down, color: Colors.redAccent),
                          style: const TextStyle(color: Colors.redAccent, fontSize: 12, fontFamily: 'monospace'),
                          onChanged: (val) {
                            setState(() { _selectedGeneratorId = val; });
                          },
                          items: generators.entries.map((e) {
                            return DropdownMenuItem(
                              value: e.key,
                              child: Text(_getGeneratorName(e.value)),
                            );
                          }).toList(),
                        ),
                      ),
                    ),
                  ],
                ),
              ),
              
              // Keyboard Area
              SizedBox(
                height: 120,
                child: _CustomVirtualKeyboard(
                  startNote: _baseKey,
                  totalKeys: _keyRange,
                  activeNotes: _activeNotes,
                  onNoteOn: _handleNoteOn,
                  onNoteOff: _handleNoteOff,
                ),
              ),
            ],
          ),
        ),
      ),
      ),
    );
  }

  Widget _buildControlKnob(String label, String value, VoidCallback onDec, VoidCallback onInc) {
    return Column(
      children: [
        Text(label, style: const TextStyle(color: Colors.grey, fontSize: 10, fontWeight: FontWeight.bold)),
        const SizedBox(height: 4),
        Row(
          children: [
            InkWell(
              onTap: onDec,
              child: const Icon(Icons.remove_circle_outline, color: Colors.grey, size: 20),
            ),
            const SizedBox(width: 8),
            Container(
              width: 50,
              alignment: Alignment.center,
              child: Text(
                value, 
                style: const TextStyle(color: Colors.cyanAccent, fontFamily: 'monospace', fontSize: 16, fontWeight: FontWeight.bold)
              ),
            ),
            const SizedBox(width: 8),
            InkWell(
              onTap: onInc,
              child: const Icon(Icons.add_circle_outline, color: Colors.grey, size: 20),
            ),
          ],
        )
      ],
    );
  }

  void _handleNoteOn(int note) async {
    setState(() => _activeNotes.add(note));
    if (_selectedGeneratorId != null) {
      try {
        await audio_api.playPreviewNoteGenerator(
          generatorId: _selectedGeneratorId!,
          noteKey: note,
          velocity: 100,
          isOn: true,
        );
      } catch (e) {
        debugPrint('Error playing note on: $e');
      }
    }
  }

  void _handleNoteOff(int note) async {
    setState(() => _activeNotes.remove(note));
    if (_selectedGeneratorId != null) {
      try {
        await audio_api.playPreviewNoteGenerator(
          generatorId: _selectedGeneratorId!,
          noteKey: note,
          velocity: 100,
          isOn: false,
        );
      } catch (e) {
        debugPrint('Error playing note off: $e');
      }
    }
  }
}

// ==== Custom Keyboard logic ====

class _CustomVirtualKeyboard extends StatelessWidget {
  final int startNote;
  final int totalKeys;
  final Set<int> activeNotes;
  final Function(int) onNoteOn;
  final Function(int) onNoteOff;

  const _CustomVirtualKeyboard({
    required this.startNote,
    required this.totalKeys,
    required this.activeNotes,
    required this.onNoteOn,
    required this.onNoteOff,
  });

  bool _isBlack(int note) {
    int n = note % 12;
    return n == 1 || n == 3 || n == 6 || n == 8 || n == 10;
  }

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        // Calculate number of white keys in the range
        int whiteKeyCount = 0;
        for (int i = 0; i < totalKeys; i++) {
          if (!_isBlack(startNote + i)) {
            whiteKeyCount++;
          }
        }
        
        if (whiteKeyCount == 0) whiteKeyCount = 1; // Prevent division by zero
        
        double whiteKeyWidth = constraints.maxWidth / whiteKeyCount;

        List<Widget> whiteKeys = [];
        List<Widget> blackKeys = [];

        int whiteIndex = 0;

        for (int i = 0; i < totalKeys; i++) {
          final note = startNote + i;
          final isBlack = _isBlack(note);
          final isPressed = activeNotes.contains(note);

          if (!isBlack) {
            whiteKeys.add(
              _PianoKey(
                note: note,
                isBlack: false,
                width: whiteKeyWidth,
                height: constraints.maxHeight,
                isPressed: isPressed,
                onNoteOn: onNoteOn,
                onNoteOff: onNoteOff,
              ),
            );
            whiteIndex++;
          } else {
            // Because our loop processes keys sequentially, if the current key is black,
            // the `whiteIndex` holds the index of the next white key in the array.
            // A black key sits between `whiteIndex - 1` and `whiteIndex`.
            // So we position it anchored to `whiteIndex * whiteKeyWidth` offset to the left.
            
            double left = (whiteIndex * whiteKeyWidth) - (whiteKeyWidth * 0.35);
            
            // Protect against left clamping if sequence starts precisely with a black key
            if (left < 0) left = 0;

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
                  onNoteOn: onNoteOn,
                  onNoteOff: onNoteOff,
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
          borderRadius: const BorderRadius.only(
            bottomLeft: Radius.circular(4),
            bottomRight: Radius.circular(4),
          ),
          boxShadow: [
            if (!isActive && widget.isBlack)
               const BoxShadow(
                color: Colors.black54,
                offset: Offset(1, 1),
                blurRadius: 1,
              )
          ],
        ),
        child: Align(
          alignment: Alignment.bottomCenter,
          child: Padding(
            padding: const EdgeInsets.only(bottom: 6.0),
            child: Text(
              _getNoteLabel(widget.note),
              style: TextStyle(
                color: isActive
                    ? Colors.black
                    : (widget.isBlack ? Colors.white54 : Colors.grey.shade600),
                fontSize: 8,
                fontWeight: FontWeight.bold,
              ),
            ),
          ),
        ),
      ),
    );
  }

  String _getNoteLabel(int note) {
    // Only show C notes label to reduce clutter unless it's the very first note
    if (note % 12 == 0) return numToMidiKey(note);
    return "";
  }
}
