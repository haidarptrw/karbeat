import 'dart:ui';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:karbeat/features/components/scrollable_virtual_keyboard.dart';
import 'package:karbeat/features/playlist/playhead.dart';
import 'package:karbeat/models/grid.dart';
import 'package:karbeat/models/piano_key.dart';
import 'package:karbeat/src/rust/api/audio.dart';
import 'package:karbeat/src/rust/api/pattern.dart';
import 'package:karbeat/state/app_state.dart';
import 'package:karbeat/utils/formatter.dart';
import 'package:karbeat/utils/logger.dart';
import 'package:linked_scroll_controller/linked_scroll_controller.dart';
import 'package:provider/provider.dart';

class PianoRollScreen extends StatefulWidget {
  final int? patternId;
  // We need the Generator ID to know which Generator to preview sound with
  final int? generatorId;

  const PianoRollScreen({super.key, this.patternId, this.generatorId});

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

  // Track active notes for Keyboard visualization
  final Set<int> _activeKeyboardNotes = {};

  int? _lastPaintedTick;
  int? _lastPaintedKey;

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
    final generatorId =
        context.read<KarbeatState>().previewGeneratorId ?? widget.generatorId;
    if (generatorId != null) {
      try {
        playPreviewNoteGenerator(
          generatorId: generatorId,
          noteKey: note,
          velocity: 100,
          isOn: true,
        );
      } catch (e) {
        KarbeatLogger.error("Failed to play preview note: $e");
      }
    }
  }

  void _handleNoteOff(int note) {
    final generatorId =
        context.read<KarbeatState>().previewGeneratorId ?? widget.generatorId;
    if (generatorId != null) {
      try {
        playPreviewNoteGenerator(
          generatorId: generatorId,
          noteKey: note,
          velocity: 100,
          isOn: false,
        );
      } catch (e) {
        KarbeatLogger.error("Failed to play preview note: $e");
      }
    }
  }

  void _handleZoom(double scale) {
    setState(() {
      _zoomX = (_zoomX * scale).clamp(0.1, 5.0);
    });
  }

  int _getSnapTicks(int denom) {
    return (960.0 * 4.0 / denom).round();
  }

  // Helper to convert int back to GridValue for the setter
  GridValue _intToGridValue(int val) {
    return GridValue.values.firstWhere(
      (e) => e.value == val,
      orElse: () => GridValue.quarter, // Default fallback
    );
  }

  void _addNoteAtOffset(Offset localPos, int patternId) {
    final state = context.read<KarbeatState>();
    final gridDenom = state.pianoRollGridDenom;

    double offsetX = localPos.dx;
    int tick = (offsetX / _zoomX).round();

    int snap = _getSnapTicks(gridDenom);
    tick = (tick / snap).round() * snap;

    double offsetY = localPos.dy;
    int keyIndex = (offsetY / _keyHeight).floor();
    int midiKey = (127 - keyIndex).clamp(0, 127);

    if (tick == _lastPaintedTick && midiKey == _lastPaintedKey) {
      return;
    }

    _lastPaintedTick = tick;
    _lastPaintedKey = midiKey;

    state.addPatternNote(
      patternId: patternId,
      key: midiKey,
      startTick: tick,
      duration: snap,
    );
  }

  void _resetPaintState() {
    _lastPaintedTick = null;
    _lastPaintedKey = null;
  }

  // void _handleVirtualKeyPan(Offset localPos) {
  //   const double whiteKeyWidth = 40.0;
  // }

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
    final selectedTool = context.select<KarbeatState, PianoRollToolSelection>(
      (s) => s.pianoRollTool,
    );

    final gridDenom = context.select<KarbeatState, int>(
      (s) => s.pianoRollGridDenom,
    );

    if (pattern == null) {
      return const Center(
        child: Text("Pattern not found", style: TextStyle(color: Colors.white)),
      );
    }

    final isDrawing = selectedTool == PianoRollToolSelection.draw;

    return Focus(
      autofocus: true,
      onKeyEvent: (node, event) {
        if (event is KeyDownEvent) {
          final note = keyMap[event.physicalKey];
          if (note != null && !_activeKeyboardNotes.contains(note)) {
            setState(() => _activeKeyboardNotes.add(note));
            _handleNoteOn(note);
            return KeyEventResult.handled;
          }
        } else if (event is KeyUpEvent) {
          final note = keyMap[event.physicalKey];
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
            patternId: pattern.id,
            name: pattern.name,
            onZoomIn: () => _handleZoom(1.2),
            onZoomOut: () => _handleZoom(1 / 1.2),
            gridDenom: gridDenom,
            onGridDenomChanged: (val) {
              if (val != null) {
                setState(() {
                  context.read<KarbeatState>().pianoRollGridDenom =
                      _intToGridValue(val);
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
                  child: ScrollConfiguration(
                    behavior: ScrollConfiguration.of(
                      context,
                    ).copyWith(scrollbars: false),
                    child: ListView.builder(
                      controller: _keysController,
                      itemCount: 128,
                      itemExtent: _keyHeight,
                      physics: const ClampingScrollPhysics(),
                      itemBuilder: (context, index) {
                        // MIDI 127 is top, 0 is bottom. List index 0 is top.
                        final midiKey = 127 - index;
                        return _PianoKey(
                          midiKey: midiKey,
                          height: _keyHeight,
                          onPlayNote: (isOn) {
                            if (widget.generatorId != null) {
                              try {
                                playPreviewNoteGenerator(
                                  generatorId: widget.generatorId!,
                                  noteKey: midiKey,
                                  velocity: 100,
                                  isOn: isOn,
                                );
                              } catch (e) {
                                KarbeatLogger.error(e.toString());
                              }
                            }
                          },
                        );
                      },
                    ),
                  ),
                ),

                // ========= GRID & NOTES & PLAYHEAD ===========
                Expanded(
                  child: Stack(
                    children: [
                      ScrollConfiguration(
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
                          physics: const AlwaysScrollableScrollPhysics(),
                          child: SingleChildScrollView(
                            controller: _gridVerticalController,
                            scrollDirection: Axis.vertical,
                            child: GestureDetector(
                              behavior: HitTestBehavior.translucent,
                              onTapDown: (details) {
                                if (isDrawing) {
                                  _addNoteAtOffset(
                                    details.localPosition,
                                    pattern.id,
                                  );
                                }
                              },
                              onPanStart: isDrawing
                                  ? (details) => _addNoteAtOffset(
                                      details.localPosition,
                                      pattern.id,
                                    )
                                  : null,
                              onPanUpdate: isDrawing
                                  ? (details) => _addNoteAtOffset(
                                      details.localPosition,
                                      pattern.id,
                                    )
                                  : null,
                              onPanEnd: isDrawing
                                  ? (details) => _resetPaintState()
                                  : null,
                              child: MouseRegion(
                                cursor: isDrawing
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
                                              gridDenom: gridDenom,
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
                                          noteId: note
                                              .id, // Pass ID instead of Index
                                          patternId: pattern.id,
                                          generatorId: widget.generatorId,
                                          zoomX: _zoomX,
                                          keyHeight: _keyHeight,
                                          selectedTool: selectedTool,
                                          snapTicks: _getSnapTicks(gridDenom),
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
                      Positioned.fill(
                        child: IgnorePointer(
                          ignoring: false,
                          child: PlayheadOverlay(
                            offsetAdjustment: 0,
                            scrollController: _gridHorizontalController,
                            onSeek: (int newSamples) {
                              // final safeSamples = newSamples < 0 ? 0 : newSamples;
                              // context.read<KarbeatState>().seekTo(safeSamples);

                              // We currently don't support seeking inside pattern playback.
                              // TODO: Add pattern playback seek implementation
                            },
                            zoomLevel: _zoomX,
                            sampleSelector: (pos) {
                              if (pos.isPatternMode) {
                                return pos.patternSamples;
                              }
                              return 0;
                            },
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),

          // ========= SCROLLABLE VIRTUAL KEYBOARD ===========
          ScrollableVirtualKeyboard(
            height: 120,
            onNoteOn: _handleNoteOn,
            onNoteOff: _handleNoteOff,
            activeNotes: _activeKeyboardNotes,
            initialCenterNote: 60,
          ),
        ],
      ),
    );
  }
}

class _PianoRollToolbar extends StatelessWidget {
  final int patternId;
  final String name;
  final VoidCallback onZoomIn;
  final VoidCallback onZoomOut;
  final int gridDenom;
  final ValueChanged<int?> onGridDenomChanged;

  const _PianoRollToolbar({
    required this.patternId,
    required this.name,
    required this.onZoomIn,
    required this.onZoomOut,
    required this.gridDenom,
    required this.onGridDenomChanged,
  });

  @override
  Widget build(BuildContext context) {
    final state = context.watch<KarbeatState>();
    final selectedTool = state.pianoRollTool;
    final generators = state.generators;
    final previewGeneratorId = state.previewGeneratorId;
    final isPlaying = state.isPlaying;

    return Container(
      height: 50,
      color: Colors.grey.shade800,
      padding: const EdgeInsets.symmetric(horizontal: 10),
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: Row(
          children: [
            // Pattern transport
            IconButton(
              icon: Icon(
                isPlaying ? Icons.stop : Icons.play_arrow,
                color: isPlaying ? Colors.orange : Colors.white70,
              ),
              onPressed: previewGeneratorId != null
                  ? () => _togglePatternPlayback(
                      context,
                      isPlaying,
                      previewGeneratorId,
                    )
                  : null,
              tooltip: isPlaying ? 'Stop' : 'Play Pattern',
              iconSize: 24,
            ),
            const SizedBox(width: 4),
            _buildDivider(),
            const SizedBox(width: 8),

            // Pattern name
            Text(
              name,
              style: const TextStyle(
                color: Colors.white,
                fontWeight: FontWeight.bold,
              ),
            ),
            const SizedBox(width: 16),
            _buildDivider(),
            const SizedBox(width: 8),

            // Tool buttons
            _ToolButton(
              icon: Icons.near_me,
              label: 'Pointer',
              isActive: selectedTool == PianoRollToolSelection.pointer,
              onTap: () =>
                  state.selectPianoRollTool(PianoRollToolSelection.pointer),
            ),
            _ToolButton(
              icon: Icons.edit,
              label: 'Draw',
              isActive: selectedTool == PianoRollToolSelection.draw,
              onTap: () =>
                  state.selectPianoRollTool(PianoRollToolSelection.draw),
            ),
            _ToolButton(
              icon: Icons.delete,
              label: 'Delete',
              isActive: selectedTool == PianoRollToolSelection.delete,
              onTap: () =>
                  state.selectPianoRollTool(PianoRollToolSelection.delete),
            ),
            _ToolButton(
              icon: Icons.crop_free,
              label: 'Select',
              isActive: selectedTool == PianoRollToolSelection.select,
              onTap: () =>
                  state.selectPianoRollTool(PianoRollToolSelection.select),
            ),
            const SizedBox(width: 8),
            _buildDivider(),
            const SizedBox(width: 8),

            // Zoom controls
            IconButton(
              icon: const Icon(Icons.zoom_in, color: Colors.white70),
              onPressed: onZoomIn,
              tooltip: 'Zoom In',
              iconSize: 20,
            ),
            IconButton(
              icon: const Icon(Icons.zoom_out, color: Colors.white70),
              onPressed: onZoomOut,
              tooltip: 'Zoom Out',
              iconSize: 20,
            ),
            const SizedBox(width: 8),

            // Grid dropdown
            DropdownButton<int>(
              value: gridDenom,
              dropdownColor: Colors.grey.shade800,
              style: const TextStyle(color: Colors.white, fontSize: 12),
              underline: const SizedBox(),
              items: const [
                DropdownMenuItem(value: 1, child: Text("1/1")),
                DropdownMenuItem(value: 2, child: Text("1/2")),
                DropdownMenuItem(value: 4, child: Text("1/4")),
                DropdownMenuItem(value: 8, child: Text("1/8")),
                DropdownMenuItem(value: 16, child: Text("1/16")),
                DropdownMenuItem(value: 32, child: Text("1/32")),
              ],
              onChanged: onGridDenomChanged,
            ),
            const SizedBox(width: 8),
            _buildDivider(),
            const SizedBox(width: 8),

            // Generator dropdown
            const Text(
              'Generator: ',
              style: TextStyle(color: Colors.white70, fontSize: 12),
            ),
            DropdownButton<int?>(
              value: previewGeneratorId,
              hint: const Text(
                'Select',
                style: TextStyle(color: Colors.white54),
              ),
              dropdownColor: Colors.grey.shade800,
              style: const TextStyle(color: Colors.white, fontSize: 12),
              underline: const SizedBox(),
              items: [
                const DropdownMenuItem<int?>(value: null, child: Text('None')),
                ...generators.entries.map(
                  (entry) => DropdownMenuItem<int?>(
                    value: entry.key,
                    child: Text(entry.value.name),
                  ),
                ),
              ],
              onChanged: (value) =>
                  state.setPreviewGenerator(generatorId: value),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildDivider() {
    return Container(width: 1, height: 30, color: Colors.grey.shade600);
  }

  void _togglePatternPlayback(
    BuildContext context,
    bool isPlaying,
    int generatorId,
  ) async {
    try {
      if (isPlaying) {
        await stopPatternPreview();
      } else {
        await playPatternPreview(
          patternId: patternId,
          generatorId: generatorId,
        );
      }
    } catch (e) {
      debugPrint('Pattern playback error: $e');
    }
  }
}

class _ToolButton extends StatelessWidget {
  final IconData icon;
  final String label;
  final bool isActive;
  final VoidCallback onTap;

  const _ToolButton({
    required this.icon,
    required this.label,
    required this.isActive,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    return Tooltip(
      message: label,
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(4),
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
          decoration: isActive
              ? BoxDecoration(
                  color: Colors.blueAccent.withAlpha(50),
                  borderRadius: BorderRadius.circular(4),
                )
              : null,
          child: Icon(
            icon,
            color: isActive ? Colors.blueAccent : Colors.white70,
            size: 20,
          ),
        ),
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

// ignore: unused_field
enum _NoteDragMode { none, move, resizeLeft, resizeRight }

class _InteractiveNote extends StatefulWidget {
  final UiNote note;
  final int noteId;
  final int patternId;
  final int? generatorId; // Optional: To preview note while dragging
  final double zoomX;
  final double keyHeight;
  final PianoRollToolSelection selectedTool;
  final int snapTicks;

  const _InteractiveNote({
    super.key,
    required this.note,
    required this.noteId,
    required this.patternId,
    this.generatorId,
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
  // double _startDragX = 0;
  // double _startDragY = 0;
  _NoteDragMode _mode = _NoteDragMode.none;

  int? _currentPreviewKey;

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

  void _playNote(int key, bool on) {
    if (widget.generatorId != null) {
      try {
        playPreviewNoteGenerator(
          generatorId: widget.generatorId!,
          noteKey: key,
          velocity: 100,
          isOn: on,
        );
      } catch (e) {
        KarbeatLogger.error(e.toString());
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    // Determine cursor based on tool
    MouseCursor cursor = SystemMouseCursors.click;
    if (widget.selectedTool == PianoRollToolSelection.delete) {
      cursor = SystemMouseCursors.basic;
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
            if (widget.selectedTool == PianoRollToolSelection.delete) {
              context.read<KarbeatState>().deletePatternNote(
                patternId: widget.patternId,
                noteId: widget.noteId,
              );
            }
          },
          onPanStart: (details) {
            if (widget.selectedTool == PianoRollToolSelection.delete) return;

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
              // _startDragX = _localLeft;
              // _startDragY = _localTop;
            });

            // Play sound on touch
            _currentPreviewKey = widget.note.key;
            _playNote(_currentPreviewKey!, true);
          },

          onPanUpdate: (details) {
            if (_mode == _NoteDragMode.none) return;
            setState(() {
              if (_mode == _NoteDragMode.move) {
                _localLeft += details.delta.dx;
                _localTop += details.delta.dy;

                int keyIndex = (_localTop / widget.keyHeight).round();
                int newKey = (127 - keyIndex).clamp(0, 127);

                if (newKey != _currentPreviewKey) {
                  if (_currentPreviewKey != null) {
                    _playNote(_currentPreviewKey!, false);
                  }
                  _currentPreviewKey = newKey;
                  _playNote(_currentPreviewKey!, true);
                }
              } else if (_mode == _NoteDragMode.resizeRight) {
                _localWidth += details.delta.dx;
                if (_localWidth < 5) _localWidth = 5;
              }
              // SUGGESTION: Implement Resize Left logic if needed (requires shifting start tick)
            });
          },
          onPanEnd: (details) {
            if (_currentPreviewKey != null) {
              _playNote(_currentPreviewKey!, false);
              _currentPreviewKey = null;
            }

            final state = context.read<KarbeatState>();

            if (widget.generatorId != null) {
              try {
                playPreviewNoteGenerator(
                  generatorId: widget.generatorId!,
                  noteKey: widget.note.key,
                  velocity: 100,
                  isOn: false,
                );
              } catch (e) {
                KarbeatLogger.error(e.toString());
              }
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
    double ticksPerGrid = 960.0 * 4.0 / gridDenom;
    double pixelsPerGrid = ticksPerGrid * zoomX;

    if (pixelsPerGrid < 4) return;

    double currentX = 0;
    int gridIndex = 0;

    while (currentX < size.width) {
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
