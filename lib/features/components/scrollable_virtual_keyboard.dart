import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:karbeat/models/piano_key.dart';

/// A horizontally scrollable virtual piano keyboard that supports all 128 MIDI notes.
/// Used for playing notes live in plugin screens and piano roll.
/// Supports:
/// - Touch/mouse interaction on keys
/// - PC keyboard input (using QWERTY layout)
/// - Horizontal scroll wheel on desktop
class ScrollableVirtualKeyboard extends StatefulWidget {
  final Function(int note) onNoteOn;
  final Function(int note) onNoteOff;
  final Set<int> activeNotes;
  final double height;
  final int?
  initialCenterNote; // MIDI note to center on initially (default: 60 = C4)

  const ScrollableVirtualKeyboard({
    super.key,
    required this.onNoteOn,
    required this.onNoteOff,
    required this.activeNotes,
    this.height = 120,
    this.initialCenterNote = 60,
  });

  @override
  State<ScrollableVirtualKeyboard> createState() =>
      _ScrollableVirtualKeyboardState();
}

class _ScrollableVirtualKeyboardState extends State<ScrollableVirtualKeyboard> {
  late ScrollController _scrollController;
  static const double _whiteKeyWidth = 40.0;
  static const int _totalNotes = 128; // MIDI notes 0-127

  // Track notes triggered by PC keyboard
  final Set<int> _keyboardActiveNotes = {};

  // Focus node for keyboard input
  final FocusNode _focusNode = FocusNode();

  @override
  void initState() {
    super.initState();
    _scrollController = ScrollController();

    // Scroll to center note after layout
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _scrollToNote(widget.initialCenterNote ?? 60);
    });
  }

  @override
  void dispose() {
    _scrollController.dispose();
    _focusNode.dispose();
    // Release any held notes on dispose
    for (final note in _keyboardActiveNotes) {
      widget.onNoteOff(note);
    }
    super.dispose();
  }

  void _scrollToNote(int midiNote) {
    // Calculate approximate scroll position for the given note
    // Count white keys up to this note
    int whiteKeysBefore = 0;
    for (int i = 0; i < midiNote; i++) {
      if (!_isBlack(i)) whiteKeysBefore++;
    }

    double targetScroll =
        (whiteKeysBefore * _whiteKeyWidth) -
        (MediaQuery.of(context).size.width / 2);
    if (targetScroll < 0) targetScroll = 0;

    if (_scrollController.hasClients) {
      _scrollController.jumpTo(targetScroll);
    }
  }

  bool _isBlack(int note) {
    int n = note % 12;
    return n == 1 || n == 3 || n == 6 || n == 8 || n == 10;
  }

  String _getNoteLabel(int note) {
    int noteIndex = note % 12;
    // Only show C notes with octave number
    if (noteIndex == 0) {
      int octave = (note ~/ 12) - 1;
      return 'C$octave';
    }
    return '';
  }

  // Handle PC keyboard key events
  KeyEventResult _handleKeyEvent(FocusNode node, KeyEvent event) {
    if (event is KeyDownEvent) {
      final note = keyMap[event.physicalKey];
      if (note != null && !_keyboardActiveNotes.contains(note)) {
        setState(() => _keyboardActiveNotes.add(note));
        widget.onNoteOn(note);
        return KeyEventResult.handled;
      }
    } else if (event is KeyUpEvent) {
      final note = keyMap[event.physicalKey];
      if (note != null) {
        setState(() => _keyboardActiveNotes.remove(note));
        widget.onNoteOff(note);
        return KeyEventResult.handled;
      }
    }
    return KeyEventResult.ignored;
  }

  // Handle horizontal scroll wheel
  void _handlePointerSignal(PointerSignalEvent event) {
    if (event is PointerScrollEvent) {
      // Use vertical scroll delta for horizontal scrolling
      final scrollDelta = event.scrollDelta.dy;
      if (_scrollController.hasClients) {
        final newOffset = _scrollController.offset + scrollDelta;
        _scrollController.jumpTo(
          newOffset.clamp(0.0, _scrollController.position.maxScrollExtent),
        );
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    // Combine external active notes with keyboard-triggered notes
    final allActiveNotes = {...widget.activeNotes, ..._keyboardActiveNotes};

    return Focus(
      focusNode: _focusNode,
      autofocus: false,
      onKeyEvent: _handleKeyEvent,
      child: GestureDetector(
        onTap: () => _focusNode.requestFocus(),
        child: Listener(
          onPointerSignal: _handlePointerSignal,
          child: Container(
            height: widget.height,
            color: Colors.black,
            padding: const EdgeInsets.symmetric(vertical: 4),
            child: LayoutBuilder(
              builder: (context, constraints) {
                // Count total white keys
                int whiteKeyCount = 0;
                for (int i = 0; i < _totalNotes; i++) {
                  if (!_isBlack(i)) whiteKeyCount++;
                }

                double totalWidth = whiteKeyCount * _whiteKeyWidth;
                double keyHeight = constraints.maxHeight;
                double blackKeyHeight = keyHeight * 0.6;
                double blackKeyWidth = _whiteKeyWidth * 0.7;

                return SingleChildScrollView(
                  controller: _scrollController,
                  scrollDirection: Axis.horizontal,
                  physics: const BouncingScrollPhysics(),
                  child: SizedBox(
                    width: totalWidth,
                    height: keyHeight,
                    child: Stack(
                      children: [
                        // White keys
                        Row(
                          children: _buildWhiteKeys(keyHeight, allActiveNotes),
                        ),
                        // Black keys (positioned)
                        ..._buildBlackKeys(
                          blackKeyWidth,
                          blackKeyHeight,
                          allActiveNotes,
                        ),
                      ],
                    ),
                  ),
                );
              },
            ),
          ),
        ),
      ),
    );
  }

  List<Widget> _buildWhiteKeys(double keyHeight, Set<int> activeNotes) {
    List<Widget> keys = [];
    for (int note = 0; note < _totalNotes; note++) {
      if (!_isBlack(note)) {
        final isPressed = activeNotes.contains(note);
        keys.add(
          _KeyWidget(
            note: note,
            isBlack: false,
            isPressed: isPressed,
            width: _whiteKeyWidth,
            height: keyHeight,
            label: _getNoteLabel(note),
            onNoteOn: widget.onNoteOn,
            onNoteOff: widget.onNoteOff,
          ),
        );
      }
    }
    return keys;
  }

  List<Widget> _buildBlackKeys(
    double blackKeyWidth,
    double blackKeyHeight,
    Set<int> activeNotes,
  ) {
    List<Widget> keys = [];
    int whiteIndex = 0;

    for (int note = 0; note < _totalNotes; note++) {
      if (_isBlack(note)) {
        final isPressed = activeNotes.contains(note);
        double left = (whiteIndex * _whiteKeyWidth) - (blackKeyWidth / 2);

        keys.add(
          Positioned(
            left: left,
            width: blackKeyWidth,
            height: blackKeyHeight,
            child: _KeyWidget(
              note: note,
              isBlack: true,
              isPressed: isPressed,
              width: blackKeyWidth,
              height: blackKeyHeight,
              label: '',
              onNoteOn: widget.onNoteOn,
              onNoteOff: widget.onNoteOff,
            ),
          ),
        );
      } else {
        whiteIndex++;
      }
    }
    return keys;
  }
}

class _KeyWidget extends StatefulWidget {
  final int note;
  final bool isBlack;
  final bool isPressed;
  final double width;
  final double height;
  final String label;
  final Function(int) onNoteOn;
  final Function(int) onNoteOff;

  const _KeyWidget({
    required this.note,
    required this.isBlack,
    required this.isPressed,
    required this.width,
    required this.height,
    required this.label,
    required this.onNoteOn,
    required this.onNoteOff,
  });

  @override
  State<_KeyWidget> createState() => _KeyWidgetState();
}

class _KeyWidgetState extends State<_KeyWidget> {
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
          border: Border.all(color: Colors.black54, width: 0.5),
          borderRadius: const BorderRadius.only(
            bottomLeft: Radius.circular(4),
            bottomRight: Radius.circular(4),
          ),
          boxShadow: widget.isBlack
              ? [
                  BoxShadow(
                    color: Colors.black.withAlpha(128),
                    blurRadius: 2,
                    offset: const Offset(0, 2),
                  ),
                ]
              : [],
        ),
        child: widget.label.isNotEmpty
            ? Align(
                alignment: Alignment.bottomCenter,
                child: Padding(
                  padding: const EdgeInsets.only(bottom: 4.0),
                  child: Text(
                    widget.label,
                    style: TextStyle(
                      color: isActive
                          ? Colors.black
                          : (widget.isBlack ? Colors.white54 : Colors.black54),
                      fontSize: 9,
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                ),
              )
            : null,
      ),
    );
  }
}
