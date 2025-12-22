import 'dart:ui';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:karbeat/features/components/virtual_keyboard.dart';
import 'package:karbeat/src/rust/api/pattern.dart';
import 'package:karbeat/state/app_state.dart';
import 'package:karbeat/utils/formatter.dart';
import 'package:karbeat/utils/logger.dart';
import 'package:linked_scroll_controller/linked_scroll_controller.dart';
import 'package:provider/provider.dart';

class PianoRollScreen extends StatefulWidget {
  final int? patternId;
  // We need the Track ID to know which Generator to preview sound with
  final int? parentTrackId;

  const PianoRollScreen({super.key, this.patternId, this.parentTrackId});

  @override
  State<StatefulWidget> createState() {
    return PianoRollScreenState();
  }
}

class PianoRollScreenState extends State<PianoRollScreen> {
  final double _keyHeight = 20.0;
  final double _keyWidth = 60.0;
  double _zoomX = 0.5;

  int _gridDenom = 4;

  late LinkedScrollControllerGroup _verticalControllers;
  late ScrollController _keysController;
  late ScrollController _gridVerticalController;
  late ScrollController _gridHorizontalController;

  // Track active notes for Keyboard visualization
  final Set<int> _activeKeyboardNotes = {};

  // Standard DAW Keyboard Mapping (Z=C3)
  static final Map<PhysicalKeyboardKey, int> _keyMap = {
    PhysicalKeyboardKey.keyZ: 48, // C4
    PhysicalKeyboardKey.keyS: 49,
    PhysicalKeyboardKey.keyX: 50,
    PhysicalKeyboardKey.keyD: 51,
    PhysicalKeyboardKey.keyC: 52,
    PhysicalKeyboardKey.keyV: 53,
    PhysicalKeyboardKey.keyG: 54,
    PhysicalKeyboardKey.keyB: 55,
    PhysicalKeyboardKey.keyH: 56,
    PhysicalKeyboardKey.keyN: 57,
    PhysicalKeyboardKey.keyJ: 58,
    PhysicalKeyboardKey.keyM: 59,
    PhysicalKeyboardKey.comma: 60, // C5
    // Upper row (Q=C4)
    PhysicalKeyboardKey.keyQ: 60,
    PhysicalKeyboardKey.digit2: 61,
    PhysicalKeyboardKey.keyW: 62,
    PhysicalKeyboardKey.digit3: 63,
    PhysicalKeyboardKey.keyE: 64,
    PhysicalKeyboardKey.keyR: 65,
    PhysicalKeyboardKey.digit5: 66,
    PhysicalKeyboardKey.keyT: 67,
    PhysicalKeyboardKey.digit6: 68,
    PhysicalKeyboardKey.keyY: 69,
    PhysicalKeyboardKey.digit7: 70,
    PhysicalKeyboardKey.keyU: 71,
    // C6
    PhysicalKeyboardKey.keyI: 72,
    PhysicalKeyboardKey.digit9: 73,
    PhysicalKeyboardKey.keyO: 74,
    PhysicalKeyboardKey.digit0: 75,
    PhysicalKeyboardKey.keyP: 76,
    PhysicalKeyboardKey.bracketLeft: 77,
    PhysicalKeyboardKey.equal: 78,
    PhysicalKeyboardKey.bracketRight: 79
  };

  @override
  void initState() {
    super.initState();
    _verticalControllers = LinkedScrollControllerGroup();
    _keysController = _verticalControllers.addAndGet();
    _gridVerticalController = _verticalControllers.addAndGet();
    _gridHorizontalController = ScrollController();

    // Jump to Middle C (MIDI 60)
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _verticalControllers.jumpTo((127 - 60) * _keyHeight - 100);
    });
  }

  @override
  void dispose() {
    _keysController.dispose();
    _gridVerticalController.dispose();
    _gridHorizontalController.dispose();
    super.dispose();
  }

  void _handleNoteOn(int note) {
    if (widget.parentTrackId != null) {
      context.read<KarbeatState>().previewNote(
        trackId: widget.parentTrackId!,
        noteKey: note,
        isOn: true,
      );
    }
  }

  void _handleNoteOff(int note) {
    if (widget.parentTrackId != null) {
      context.read<KarbeatState>().previewNote(
        trackId: widget.parentTrackId!,
        noteKey: note,
        isOn: false,
      );
    }
  }

  void _handleAddNote(TapDownDetails details, int patternId) {
    final state = context.read<KarbeatState>();
    if (state.selectedTool == ToolSelection.delete) return;

    double offsetX = details.localPosition.dx;
    int tick = (offsetX / _zoomX).round();

    int snap = _getSnapTicks();
    tick = (tick / snap).round() * snap;

    // Calculate key
    double offsetY = details.localPosition.dy;
    int keyIndex = (offsetY / _keyHeight).floor();
    int midiKey = (127 - keyIndex).clamp(0, 127);

    context.read<KarbeatState>().addPatternNote(
      patternId: patternId,
      key: midiKey,
      startTick: tick,
      duration: snap,
    );
  }

  void _handleZoom(double scale) {
    setState(() {
      _zoomX = (_zoomX * scale).clamp(0.1, 5.0);
    });
  }

  int _getSnapTicks() {
    return (960.0 / 4.0 / _gridDenom).round();
  }

  @override
  Widget build(BuildContext context) {
    if (widget.patternId == null) {
      return const Center(
        child: Text(
          "No Pattern Selected",
          style: TextStyle(color: Colors.white),
        ),
      );
    }

    final pattern = context.select<KarbeatState, UiPattern?>(
      (s) => s.patterns[widget.patternId],
    );

    // Also listen to selected tool for cursor updates on the grid
    final selectedTool = context.select<KarbeatState, ToolSelection>(
      (s) => s.selectedTool,
    );

    if (pattern == null) {
      return const Center(
        child: Text("Pattern not found", style: TextStyle(color: Colors.white)),
      );
    }

    return Focus(
      autofocus: true,
      onKeyEvent: (node, event) {
        if (event is KeyDownEvent) {
          final note = _keyMap[event.physicalKey];
          if (note != null && !_activeKeyboardNotes.contains(note)) {
            setState(() => _activeKeyboardNotes.add(note));
            _handleNoteOn(note);
            return KeyEventResult.handled;
          }
        } else if (event is KeyUpEvent) {
          final note = _keyMap[event.physicalKey];
          if (note != null) {
            setState(() => _activeKeyboardNotes.remove(note));
            _handleNoteOff(note);
            return KeyEventResult.handled;
          }
        }
        return KeyEventResult.ignored;
      },
      child: Column(
        children: [
          // ==== TOOLBAR ===
          _PianoRollToolbar(
            name: pattern.name,
            onZoomIn: () => _handleZoom(1.2),
            onZoomOut: () => _handleZoom(1 / 1.2),
            gridDenom: _gridDenom,
            onGridDenomChanged: (val) {
              if (val != null) {
                setState(() {
                  _gridDenom = val;
                });
              }
            },
          ),

          // === EDITOR AREA ===
          Expanded(
            child: Row(
              children: [
                // PIANO KEYS (Left)
                SizedBox(
                  width: _keyWidth,
                  child: ListView.builder(
                    controller: _keysController,
                    itemCount: 128,
                    itemExtent: _keyHeight,
                    itemBuilder: (context, index) {
                      // MIDI 127 is top, 0 is bottom. List index 0 is top.
                      final midiKey = 127 - index;
                      return _PianoKey(
                        midiKey: midiKey,
                        height: _keyHeight,
                        onPlayNote: (isOn) {
                          if (widget.parentTrackId != null) {
                            context.read<KarbeatState>().previewNote(
                              trackId: widget.parentTrackId!,
                              noteKey: midiKey,
                              isOn: isOn,
                            );
                          }
                        },
                      );
                    },
                  ),
                ),

                // GRID & NOTES
                Expanded(
                  child: ScrollConfiguration(
                    behavior: ScrollConfiguration.of(context).copyWith(
                      scrollbars: true,
                      dragDevices: {
                        PointerDeviceKind.touch,
                        PointerDeviceKind.mouse,
                      },
                    ),
                    child: SingleChildScrollView(
                      controller: _gridHorizontalController,
                      scrollDirection: Axis.horizontal,
                      child: SingleChildScrollView(
                        controller: _gridVerticalController,
                        scrollDirection: Axis.vertical,
                        child: GestureDetector(
                          behavior: HitTestBehavior.translucent,
                          onDoubleTapDown: (details) =>
                              _handleAddNote(details, pattern.id),
                          child: MouseRegion(
                            cursor: selectedTool == ToolSelection.draw
                                ? SystemMouseCursors.copy
                                : SystemMouseCursors.basic,
                            child: SizedBox(
                              height: 128 * _keyHeight,
                              width:
                                  pattern.lengthTicks * _zoomX +
                                  1000, // Approx width
                              child: Stack(
                                children: [
                                  // Grid background
                                  Positioned.fill(
                                    child: RepaintBoundary(
                                      child: CustomPaint(
                                        painter: _PianoGridPainter(
                                          zoomX: _zoomX,
                                          keyHeight: _keyHeight,
                                          gridDenom: _gridDenom,
                                        ),
                                      ),
                                    ),
                                  ),

                                  // LAYER B: Interactive Notes
                                  ...pattern.notes.map((note) {
                                    return _InteractiveNote(
                                      // Use note ID as the Flutter Key for efficient diffing
                                      key: ValueKey(note.id),
                                      note: note,
                                      noteId:
                                          note.id, // Pass ID instead of Index
                                      patternId: pattern.id,
                                      trackId: widget.parentTrackId,
                                      zoomX: _zoomX,
                                      keyHeight: _keyHeight,
                                      selectedTool: selectedTool,
                                      snapTicks: _getSnapTicks(),
                                    );
                                  }),
                                ],
                              ),
                            ),
                          ),
                        ),
                      ),
                    ),
                  ),
                ),
              ],
            ),
          ),

          // ========= VIRTUAL KEYBOARD ===========
          SizedBox(
            height: 120,
            child: Container(
              color: Colors.black,
              padding: const EdgeInsets.symmetric(vertical: 4),
              child: VirtualKeyboard(
                startOctave: 4,
                octaveCount: 2,
                onNoteOn: _handleNoteOn,
                onNoteOff: _handleNoteOff,
                activeNotes: _activeKeyboardNotes,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _PianoRollToolbar extends StatelessWidget {
  final String name;
  final VoidCallback onZoomIn;
  final VoidCallback onZoomOut;
  final int gridDenom;
  final ValueChanged<int?> onGridDenomChanged;

  const _PianoRollToolbar({
    required this.name,
    required this.onZoomIn,
    required this.onZoomOut,
    required this.gridDenom,
    required this.onGridDenomChanged,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      height: 40,
      color: Colors.grey.shade800,
      padding: const EdgeInsets.symmetric(horizontal: 10),
      child: Row(
        children: [
          Text(
            name,
            style: const TextStyle(
              color: Colors.white,
              fontWeight: FontWeight.bold,
            ),
          ),
          const SizedBox(width: 20),
          IconButton(
            icon: const Icon(Icons.zoom_in, color: Colors.white70),
            onPressed: onZoomIn,
          ),
          IconButton(
            icon: const Icon(Icons.zoom_out, color: Colors.white70),
            onPressed: onZoomOut,
          ),
          const SizedBox(width: 20),
          DropdownButton<int>(
            value: gridDenom,
            dropdownColor: Colors.grey.shade800,
            style: const TextStyle(color: Colors.white),
            items: const [
              DropdownMenuItem(value: 1, child: Text("1/1 Bar")),
              DropdownMenuItem(value: 2, child: Text("1/2 Note")),
              DropdownMenuItem(value: 4, child: Text("1/4 Beat")),
              DropdownMenuItem(value: 8, child: Text("1/8 Note")),
              DropdownMenuItem(value: 16, child: Text("1/16 Note")),
            ],
            onChanged: onGridDenomChanged,
          ),
        ],
      ),
    );
  }
}

class _PianoKey extends StatefulWidget {
  final int midiKey;
  final Function(bool) onPlayNote;
  final double height;

  const _PianoKey({
    required this.midiKey,
    required this.height,
    required this.onPlayNote,
  });

  @override
  State<_PianoKey> createState() {
    return _PianoKeyState();
  }
}

class _PianoKeyState extends State<_PianoKey> {
  bool _isPressed = false;
  @override
  Widget build(BuildContext context) {
    const blackIndices = [1, 3, 6, 8, 10];
    final isBlack = blackIndices.contains(widget.midiKey % 12);
    final label = numToMidiKey(widget.midiKey);

    return Listener(
      onPointerDown: (event) {
        setState(() {
          _isPressed = true;
        });
        widget.onPlayNote(true);
      },
      onPointerUp: (event) {
        setState(() {
          _isPressed = false;
        });
        widget.onPlayNote(false);
      },
      onPointerCancel: (event) {
        if (_isPressed) {
          setState(() {
            _isPressed = false;
          });
          widget.onPlayNote(false);
        }
      },
      child: Container(
        height: widget.height,
        decoration: BoxDecoration(
          color: _isPressed
              ? Colors.cyanAccent
              : (isBlack ? Colors.black : Colors.white),
          border: Border(
            bottom: BorderSide(color: Colors.grey.shade700, width: 0.5),
          ),
        ),
        alignment: Alignment.centerRight,
        padding: const EdgeInsets.only(right: 4),
        child: (widget.midiKey % 12 == 0 || !isBlack)
            ? Text(
                label,
                style: TextStyle(
                  fontSize: 9,
                  color: _isPressed
                      ? Colors.black
                      : (isBlack ? Colors.white54 : Colors.black54),
                ),
              )
            : null,
      ),
    );
  }
}

enum _NoteDragMode { none, move, resizeLeft, resizeRight }

class _InteractiveNote extends StatefulWidget {
  final UiNote note;
  final int noteId;
  final int patternId;
  final int? trackId; // Optional: To preview note while dragging
  final double zoomX;
  final double keyHeight;
  final ToolSelection selectedTool;
  final int snapTicks;

  const _InteractiveNote({
    super.key,
    required this.note,
    required this.noteId,
    required this.patternId,
    this.trackId,
    required this.zoomX,
    required this.keyHeight,
    required this.selectedTool,
    required this.snapTicks,
  });

  @override
  State<_InteractiveNote> createState() {
    return _InteractiveNoteState();
  }
}

class _InteractiveNoteState extends State<_InteractiveNote> {
  late double _localLeft;
  late double _localWidth;
  late double _localTop;
  double _startDragX = 0;
  double _startDragY = 0;
  _NoteDragMode _mode = _NoteDragMode.none;

  @override
  void initState() {
    super.initState();
    _syncFromProps();
  }

  @override
  void didUpdateWidget(covariant _InteractiveNote oldWidget) {
    super.didUpdateWidget(oldWidget);
    // Only sync if we are NOT dragging to avoid jitter
    if (_mode == _NoteDragMode.none) {
      _syncFromProps();
    }
  }

  void _syncFromProps() {
    _localLeft = widget.note.startTick * widget.zoomX;
    _localWidth = widget.note.duration * widget.zoomX;
    _localTop = (127 - widget.note.key) * widget.keyHeight;
  }

  @override
  Widget build(BuildContext context) {
    // Determine cursor based on tool
    MouseCursor cursor = SystemMouseCursors.click;
    if (widget.selectedTool == ToolSelection.delete) {
      cursor = SystemMouseCursors.basic; // Or a delete icon if available
    } else if (_mode != _NoteDragMode.none) {
      cursor = SystemMouseCursors.grabbing;
    } else {
      cursor = SystemMouseCursors.click;
    }

    return Positioned(
      top: _localTop + 1,
      left: _localLeft,
      width: _localWidth < 5 ? 5 : _localWidth,
      height: widget.keyHeight - 2,
      child: MouseRegion(
        cursor: cursor,
        child: GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTap: () {
            if (widget.selectedTool == ToolSelection.delete) {
              context.read<KarbeatState>().deletePatternNote(
                patternId: widget.patternId,
                noteId: widget.noteId,
              );
            }
          },
          onDoubleTap: () {
            context.read<KarbeatState>().deletePatternNote(
              patternId: widget.patternId,
              noteId: widget.noteId,
            );
          },
          onPanStart: (details) {
            if (widget.selectedTool == ToolSelection.delete) return;

            final renderBox = context.findRenderObject() as RenderBox;
            final localPos = renderBox.globalToLocal(details.globalPosition);

            // Hit test edge for resizing
            const edgeThreshold = 10.0;

            setState(() {
              if (localPos.dx > _localWidth - edgeThreshold) {
                _mode = _NoteDragMode.resizeRight;
              } else {
                _mode = _NoteDragMode.move;
              }
              _startDragX = _localLeft;
              _startDragY = _localTop;
            });

            // Play sound on touch
            if (widget.trackId != null) {
              context.read<KarbeatState>().previewNote(
                trackId: widget.trackId!,
                noteKey: widget.note.key,
                isOn: true,
              );
            }
          },

          onPanUpdate: (details) {
            if (_mode == _NoteDragMode.none) return;
            setState(() {
              if (_mode == _NoteDragMode.move) {
                _localLeft += details.delta.dx;
                _localTop += details.delta.dy;

                // Visual snapping to Y grid (Key)
                // _localTop =
                //     (_localTop / widget.keyHeight).round() * widget.keyHeight;
              } else if (_mode == _NoteDragMode.resizeRight) {
                _localWidth += details.delta.dx;
                if (_localWidth < 5) _localWidth = 5;
              }
              // SUGGESTION: Implement Resize Left logic if needed (requires shifting start tick)
            });
          },
          onPanEnd: (details) {
            final state = context.read<KarbeatState>();

            if (widget.trackId != null) {
              state.previewNote(
                trackId: widget.trackId!,
                noteKey: widget.note.key,
                isOn: false,
              );
            }

            int snap = widget.snapTicks;

            if (_mode == _NoteDragMode.move) {
              int keyIndex = (_localTop / widget.keyHeight).round();
              int newKey = (127 - keyIndex).clamp(0, 127);

              // snap time
              int rawTick = (_localLeft / widget.zoomX).round();

              int newStartTick = (rawTick / snap).round() * snap;
              if (newStartTick < 0) newStartTick = 0;

              state.movePatternNote(
                patternId: widget.patternId,
                noteId: widget.noteId,
                newStartTick: newStartTick,
                newKey: newKey,
              );
            } else if (_mode == _NoteDragMode.resizeRight) {
              int rawDuration = (_localWidth / widget.zoomX).round();
              int newDuration = (rawDuration / snap).round() * snap;
              if (newDuration < 10) newDuration = snap;

              state.resizePatternNote(
                patternId: widget.patternId,
                noteId: widget.noteId,
                newDuration: newDuration,
              );
            }
            setState(() {
              _mode = _NoteDragMode.none;
              // Snap visual state immediately to grid so it looks clean
              // _localTop =
              //     (_localTop / widget.keyHeight).round() * widget.keyHeight;
            });
          },
          child: Container(
            decoration: BoxDecoration(
              color: _mode != _NoteDragMode.none
                  ? Colors.pink
                  : Colors.pinkAccent,
              borderRadius: BorderRadius.circular(2),
              border: Border.all(color: Colors.white30),
            ),
            child: _localWidth > 30
                ? const Center(
                    child: Icon(
                      Icons.drag_handle,
                      size: 12,
                      color: Colors.white24,
                    ),
                  )
                : null,
          ),
        ),
      ),
    );
  }
}

class _PianoGridPainter extends CustomPainter {
  final double zoomX;
  final double keyHeight;
  final int gridDenom;

  _PianoGridPainter({
    required this.zoomX,
    required this.keyHeight,
    required this.gridDenom,
  });

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()..strokeWidth = 1.0;

    // Horizontal Lines (Keys)
    paint.color = Colors.white10;
    for (int i = 0; i < 128; i++) {
      final y = i * keyHeight;
      canvas.drawLine(Offset(0, y), Offset(size.width, y), paint);
    }

    // Vertical Lines (Grid)
    // 960 ticks = 1 Beat (1/4 Note)
    // Ticks per grid line:
    double ticksPerGrid = 960.0 * 4.0 / gridDenom;
    double pixelsPerGrid = ticksPerGrid * zoomX;

    if (pixelsPerGrid < 4) return;

    double currentX = 0;
    int gridIndex = 0;

    while (currentX < size.width) {
      // Determine line strength
      // Beat lines (every 1/4 note) are stronger
      // Bar lines (every 4 beats) are strongest

      bool isBeat = (gridIndex * ticksPerGrid) % 960.0 == 0;
      bool isBar = (gridIndex * ticksPerGrid) % (960.0 * 4.0) == 0;

      if (isBar) {
        paint.color = Colors.white54;
        paint.strokeWidth = 1.5;
      } else if (isBeat) {
        paint.color = Colors.white24;
        paint.strokeWidth = 1.0;
      } else {
        paint.color = Colors.white10;
        paint.strokeWidth = 0.5;
      }

      canvas.drawLine(
        Offset(currentX, 0),
        Offset(currentX, size.height),
        paint,
      );

      currentX += pixelsPerGrid;
      gridIndex++;
    }
  }

  @override
  bool shouldRepaint(covariant _PianoGridPainter old) =>
      old.zoomX != zoomX || old.gridDenom != gridDenom;
}
