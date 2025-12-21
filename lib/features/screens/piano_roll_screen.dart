import 'dart:ui';

import 'package:flutter/material.dart';
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

  late LinkedScrollControllerGroup _verticalControllers;
  late ScrollController _keysController;
  late ScrollController _gridVerticalController;
  late ScrollController _gridHorizontalController;

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

  void _handleAddNote(TapDownDetails details, int patternId) {
    double offsetX = details.localPosition.dx;
    int tick = (offsetX / _zoomX).round();
    int snap = 240; // snap to grid (240 ticks)
    tick = (tick / snap).round() * snap;

    // Calculate key
    double offsetY = details.localPosition.dy;
    int keyIndex = (offsetY / _keyHeight).floor();
    int midiKey = (127 - keyIndex).clamp(0, 127);

    context.read<KarbeatState>().addPatternNote(
      patternId: patternId,
      key: midiKey,
      startTick: tick,
      duration: 480,
    );
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

    if (pattern == null) {
      return const Center(
        child: Text("Pattern not found", style: TextStyle(color: Colors.white)),
      );
    }

    return Column(
      children: [
        // ==== TOOLBAR ===
        _PianoRollToolbar(
          name: "Toolbar",
          onZoomIn: () => setState(() => _zoomX *= 1.2),
          onZoomOut: () => setState(() => _zoomX /= 1.2),
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
                        onDoubleTapDown: (details) =>
                            _handleAddNote(details, pattern.id),
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
                                    ),
                                  ),
                                ),
                              ),
                              // Notes
                              // LAYER B: Interactive Notes
                              // We use .asMap() to get the index, which is required
                              // for the Rust API (resize_note/delete_note takes index)
                              ...pattern.notes.map((note) {
                                return _InteractiveNote(
                                  // Use note ID as the Flutter Key for efficient diffing
                                  key: ValueKey(note.id),
                                  note: note,
                                  noteId: note.id, // Pass ID instead of Index
                                  patternId: pattern.id,
                                  trackId: widget.parentTrackId,
                                  zoomX: _zoomX,
                                  keyHeight: _keyHeight,
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
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildNoteWidget(UiNote note) {
    final top = (127 - note.key) * _keyHeight;
    final left = note.startTick * _zoomX;
    final width = note.duration * _zoomX;

    return Positioned(
      top: top + 1,
      left: left,
      width: width < 5 ? 5 : width,
      height: _keyHeight - 2,
      child: GestureDetector(
        onTap: () {
          // Select note logic
        },
        child: Container(
          decoration: BoxDecoration(
            color: Colors.pinkAccent,
            borderRadius: BorderRadius.circular(2),
            border: Border.all(color: Colors.white30),
          ),
        ),
      ),
    );
  }
}

class _PianoRollToolbar extends StatelessWidget {
  final String name;
  final VoidCallback onZoomIn;
  final VoidCallback onZoomOut;

  const _PianoRollToolbar({
    required this.name,
    required this.onZoomIn,
    required this.onZoomOut,
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
          const Spacer(),
          IconButton(
            icon: const Icon(Icons.zoom_in, color: Colors.white70),
            onPressed: onZoomIn,
          ),
          IconButton(
            icon: const Icon(Icons.zoom_out, color: Colors.white70),
            onPressed: onZoomOut,
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

  const _InteractiveNote({
    super.key,
    required this.note,
    required this.noteId,
    required this.patternId,
    this.trackId,
    required this.zoomX,
    required this.keyHeight,
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
    return Positioned(
      top: _localTop + 1,
      left: _localLeft,
      width: _localWidth < 5 ? 5 : _localWidth,
      height: widget.keyHeight - 2,
      child: MouseRegion(
        cursor: _mode == _NoteDragMode.none
            ? SystemMouseCursors.click
            : SystemMouseCursors.grabbing,
        child: GestureDetector(
          behavior: HitTestBehavior.opaque,
          onDoubleTap: () {
            context.read<KarbeatState>().deletePatternNote(
              patternId: widget.patternId,
              noteId: widget.noteId,
            );
          },
          onPanStart: (details) {
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
            setState(() {
              if (_mode == _NoteDragMode.move) {
                _localLeft += details.delta.dx;
                _localTop += details.delta.dy;

                // Visual snapping to Y grid (Key)
                _localTop =
                    (_localTop / widget.keyHeight).round() * widget.keyHeight;
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
              context.read<KarbeatState>().previewNote(
                trackId: widget.trackId!,
                noteKey: widget.note.key,
                isOn: false,
              );
            }

            if (_mode == _NoteDragMode.move) {
              int keyIndex = (_localTop / widget.keyHeight).round();
              int newKey = (127 - keyIndex).clamp(0, 127);

              int newStartTick = (_localLeft / widget.zoomX).round();
              if (newStartTick < 0) newStartTick = 0;

              state.movePatternNote(
                patternId: widget.patternId,
                noteId: widget.noteId,
                newStartTick: newStartTick,
                newKey: newKey,
              );
            } else if (_mode == _NoteDragMode.resizeRight) {
              int newDuration = (_localWidth / widget.zoomX).round();
              if (newDuration < 10) newDuration = 10;

              state.resizePatternNote(
                patternId: widget.patternId,
                noteId: widget.noteId,
                newDuration: newDuration,
              );
            }
            setState(() {
              _mode = _NoteDragMode.none;
              _localTop =
                  (_localTop / widget.keyHeight).round() * widget.keyHeight;
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

  _PianoGridPainter({required this.zoomX, required this.keyHeight});

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()..strokeWidth = 1.0;

    // Horizontal Lines (Keys)
    paint.color = Colors.white10;
    for (int i = 0; i < 128; i++) {
      final y = i * keyHeight;
      canvas.drawLine(Offset(0, y), Offset(size.width, y), paint);
    }

    // Vertical Lines (Beats)
    // 960 ticks = 1 beat
    final ticksPerBeat = 960 * zoomX;
    final double pixelsPerBeat = ticksPerBeat * zoomX;

    if (pixelsPerBeat < 2) return;

    for (double x = 0; x < size.width; x += pixelsPerBeat) {
      int beatIndex = (x / pixelsPerBeat).round();
      paint.color = (beatIndex % 4 == 0)
          ? Colors.white24
          : Colors.white10; // Bar vs Beat
      canvas.drawLine(Offset(x, 0), Offset(x, size.height), paint);
    }
  }

  @override
  bool shouldRepaint(covariant _PianoGridPainter old) => old.zoomX != zoomX;
}
