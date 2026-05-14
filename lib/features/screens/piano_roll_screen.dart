import 'dart:async';
import 'dart:math';
import 'dart:ui';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:karbeat/features/components/context_menu.dart';
import 'package:karbeat/features/components/scrollable_virtual_keyboard.dart';
import 'package:karbeat/models/grid.dart';
import 'package:karbeat/models/piano_key.dart';
import 'package:karbeat/src/rust/api/audio.dart';
import 'package:karbeat/src/rust/api/pattern.dart';
import 'package:karbeat/src/rust/api/project.dart';
import 'package:karbeat/src/rust/api/transport.dart';
import 'package:karbeat/state/app_state.dart';
import 'package:karbeat/utils/formatter.dart';
import 'package:karbeat/utils/logger.dart';
import 'package:linked_scroll_controller/linked_scroll_controller.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

class PianoRollScreen extends ConsumerStatefulWidget {
  final int? patternId;
  // We need the Generator ID to know which Generator to preview sound with
  final int? generatorId;

  const PianoRollScreen({super.key, this.patternId, this.generatorId});

  @override
  ConsumerState<PianoRollScreen> createState() {
    return PianoRollScreenState();
  }
}

class PianoRollScreenState extends ConsumerState<PianoRollScreen> {
  final double _keyHeight = 20.0;
  final double _keyWidth = 60.0;

  late LinkedScrollControllerGroup _verticalControllers;
  late ScrollController _keysController;
  late ScrollController _gridVerticalController;
  late ScrollController _gridHorizontalController;

  // Track active notes for Keyboard visualization
  final Set<int> _activeKeyboardNotes = {};

  final Set<String> _recentlyAddedNotes = {};
  final List<(int, int, int)> _brushAddNotes = []; // (key, startTick, duration)
  final Set<int> _brushDeleteNoteIds = {};
  Timer? _autoScrollTimer;
  double _lastZoomDragY = 0;
  Offset? _selectionStart;
  Offset? _selectionEnd;
  Offset? _lastInteractionPos;

  @override
  void initState() {
    super.initState();
    _verticalControllers = LinkedScrollControllerGroup();
    _keysController = _verticalControllers.addAndGet();
    _gridVerticalController = _verticalControllers.addAndGet();
    _gridHorizontalController = ScrollController();

    // Jump to Middle C (MIDI 72)
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _verticalControllers.jumpTo((127 - 72) * _keyHeight - 100);
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
        ref.read(globalStateProvider).previewGeneratorId ?? widget.generatorId;
    if (generatorId != null) {
      try {
        playPreviewNoteGenerator(
          generatorId: generatorId,
          noteKey: note,
          velocity: 100,
          isOn: true,
        );
      } catch (e) {
        AppLogger.error("Failed to play preview note: $e");
      }
    }
  }

  void _handleNoteOff(int note) {
    final generatorId =
        ref.read(globalStateProvider).previewGeneratorId ?? widget.generatorId;
    if (generatorId != null) {
      try {
        playPreviewNoteGenerator(
          generatorId: generatorId,
          noteKey: note,
          velocity: 100,
          isOn: false,
        );
      } catch (e) {
        AppLogger.error("Failed to play preview note: $e");
      }
    }
  }

  void _handleZoom(double scale) {
    final state = ref.read(globalStateProvider);
    final newZoom =
        state.zoomLevelTick *
        scale; // we not clamp here because the zoomLevelTick setter already handle clamping

    // Only update if the value actually changed
    if (state.zoomLevelTick != newZoom) {
      state.zoomLevelTick = newZoom;
      // The state provider *should* trigger a rebuild via notifyListeners(),
      // but to guarantee the local pointer event updates the UI instantly:
      setState(() {});
    }
  }

  int _getSnapTicks(GridSize denom) {
    return (960.0 * 4.0 / denom.value).round();
  }

  void _handleBrushAdd(Offset localPos) {
    final state = ref.read(globalStateProvider);
    final gridDenom = state.pianoRollGridDenom;
    final zoomX = state.zoomLevelTick;

    int tick = (localPos.dx / zoomX).round();
    int snap = _getSnapTicks(gridDenom);
    tick = (tick / snap).round() * snap;

    int keyIndex = (localPos.dy / _keyHeight).floor();
    int midiKey = (127 - keyIndex).clamp(0, 127);

    final noteKeyStr = "${tick}_$midiKey";
    if (_recentlyAddedNotes.contains(noteKeyStr)) return;

    setState(() {
      _recentlyAddedNotes.add(noteKeyStr);
      _brushAddNotes.add((midiKey, tick, snap));
    });
  }

  void _submitBrushAdd(int patternId) {
    if (_brushAddNotes.isNotEmpty) {
      ref
          .read(globalStateProvider)
          .addPatternNoteBatch(
            patternId: patternId,
            notes: List.from(_brushAddNotes),
          );
      setState(() {
        _brushAddNotes.clear();
        _recentlyAddedNotes.clear();
      });
    }
  }

  void _handleBrushDelete(Offset localPos, UiPattern pattern) {
    final zoomX = ref.read(globalStateProvider).zoomLevelTick;
    int tick = (localPos.dx / zoomX).round();
    int keyIndex = (localPos.dy / _keyHeight).floor();
    int midiKey = (127 - keyIndex).clamp(0, 127);

    bool added = false;
    for (final note in pattern.notes) {
      if (note.key == midiKey &&
          tick >= note.startTick &&
          tick < note.startTick + note.duration) {
        if (!_brushDeleteNoteIds.contains(note.id)) {
          _brushDeleteNoteIds.add(note.id);
          added = true;
        }
      }
    }

    if (added) {
      setState(() {});
    }
  }

  void _submitBrushDelete(int patternId) {
    if (_brushDeleteNoteIds.isNotEmpty) {
      ref
          .read(globalStateProvider)
          .deletePatternNoteBatch(
            patternId: patternId,
            noteIds: _brushDeleteNoteIds.toList(),
          );
      setState(() {
        _brushDeleteNoteIds.clear();
      });
    }
  }

  void _resetPaintState() {
    _recentlyAddedNotes.clear();
    _brushAddNotes.clear();
    _brushDeleteNoteIds.clear();
  }

  void _handleHorizontalAutoScroll(double globalX) {
    const double edgeThreshold = 50.0;
    const double maxSpeed = 14.0;

    // Width of the piano-key sidebar so we can compute the grid's screen rect
    final double gridLeft = _keyWidth; // same constant used for SizedBox width
    final double screenWidth = MediaQuery.of(context).size.width;

    double dx = 0;
    if (globalX < gridLeft + edgeThreshold) {
      // Near / past left edge — scroll left, faster the closer to the edge
      final ratio = ((gridLeft + edgeThreshold) - globalX) / edgeThreshold;
      dx = -maxSpeed * ratio.clamp(0.0, 1.0);
    } else if (globalX > screenWidth - edgeThreshold) {
      // Near / past right edge — scroll right
      final ratio = (globalX - (screenWidth - edgeThreshold)) / edgeThreshold;
      dx = maxSpeed * ratio.clamp(0.0, 1.0);
    }

    if (dx != 0) {
      if (_gridHorizontalController.hasClients) {
        _gridHorizontalController.jumpTo(
          (_gridHorizontalController.offset + dx).clamp(
            0.0,
            _gridHorizontalController.position.maxScrollExtent,
          ),
        );
      }
    }
  }

  void _handleAutoScroll(Offset globalPos, Size screenSize) {
    const double edgeThreshold = 40.0;
    const double scrollSpeed = 10.0;
    final double gridLeft = _keyWidth;

    double dx = 0;
    double dy = 0;

    if (globalPos.dx < gridLeft + edgeThreshold) {
      dx = -scrollSpeed;
    } else if (globalPos.dx > screenSize.width - edgeThreshold) {
      dx = scrollSpeed;
    }

    if (globalPos.dy < edgeThreshold) {
      dy = -scrollSpeed;
    } else if (globalPos.dy > screenSize.height - edgeThreshold) {
      dy = scrollSpeed;
    }

    if (dx != 0 || dy != 0) {
      if (_autoScrollTimer?.isActive ?? false) return;
      _autoScrollTimer = Timer.periodic(const Duration(milliseconds: 16), (
        timer,
      ) {
        if (dx != 0 && _gridHorizontalController.hasClients) {
          _gridHorizontalController.jumpTo(
            (_gridHorizontalController.offset + dx).clamp(
              0.0,
              _gridHorizontalController.position.maxScrollExtent,
            ),
          );
        }
        if (dy != 0 && _gridVerticalController.hasClients) {
          _gridVerticalController.jumpTo(
            (_gridVerticalController.offset + dy).clamp(
              0.0,
              _gridVerticalController.position.maxScrollExtent,
            ),
          );
        }
      });
    } else {
      _stopAutoScroll();
    }
  }

  void _stopAutoScroll() {
    _autoScrollTimer?.cancel();
    _autoScrollTimer = null;
  }

  // void _handleVirtualKeyPan(Offset localPos) {
  //   const double whiteKeyWidth = 40.0;
  // }

  void _commitRangeSelection(UiPattern pattern) {
    if (_selectionStart == null || _selectionEnd == null) return;

    final state = ref.read(globalStateProvider);
    final zoomX = state.zoomLevelTick;
    final rect = Rect.fromPoints(_selectionStart!, _selectionEnd!);

    Set<int> selectedIds = {};

    for (final note in pattern.notes) {
      double noteLeft = note.startTick * zoomX;
      double noteTop = (127 - note.key) * _keyHeight;
      double noteWidth = note.duration * zoomX;

      Rect noteRect = Rect.fromLTWH(noteLeft, noteTop, noteWidth, _keyHeight);

      // If the selection box overlaps the note's calculated rectangle
      if (rect.overlaps(noteRect)) {
        selectedIds.add(note.id);
      }
    }

    if (selectedIds.isNotEmpty) {
      state.selectNotes(selectedIds);
    } else {
      state.clearNoteSelection(); // Clicking empty space clears selection
    }

    setState(() {
      _selectionStart = null;
      _selectionEnd = null;
    });
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

    final state = ref.watch(globalStateProvider);
    final pattern = state.patterns[widget.patternId];
    final zoomX = state.zoomLevelTick;
    final selectedTool = state.pianoRollTool;
    final gridDenom = state.pianoRollGridDenom;

    final selectedNoteIds = state.selectedNoteIds;
    bool isInteracting = false;

    if (pattern == null) {
      return const Center(
        child: Text("Pattern not found", style: TextStyle(color: Colors.white)),
      );
    }

    // Filter notes into separate layers
    final selectedNotes = pattern.notes
        .where((n) => selectedNoteIds.contains(n.id))
        .toList();
    final unselectedNotes = pattern.notes
        .where((n) => !selectedNoteIds.contains(n.id))
        .toList();

    final isDrawing = selectedTool == PianoRollToolSelection.draw;
    final isDeleting = selectedTool == PianoRollToolSelection.delete;
    final isPan = selectedTool == PianoRollToolSelection.pan;
    final isZoom = selectedTool == PianoRollToolSelection.zoom;
    final isSelecting = selectedTool == PianoRollToolSelection.select;

    return Stack(
      children: [
        Focus(
          autofocus: false,
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
                      ref.read(globalStateProvider).pianoRollGridDenom = val;
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
                          physics: isPan
                              ? const ClampingScrollPhysics()
                              : const NeverScrollableScrollPhysics(),
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
                                    AppLogger.error(e.toString());
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
                              physics: isPan
                                  ? const AlwaysScrollableScrollPhysics()
                                  : const NeverScrollableScrollPhysics(),
                              child: SingleChildScrollView(
                                controller: _gridVerticalController,
                                scrollDirection: Axis.vertical,
                                physics: isPan
                                    ? const AlwaysScrollableScrollPhysics()
                                    : const NeverScrollableScrollPhysics(),
                                child: Listener(
                                  onPointerDown: (event) {
                                    _lastInteractionPos = event.localPosition;
                                    if (isZoom || isPan) {
                                      setState(() => isInteracting = true);
                                    }

                                    if (isDrawing) {
                                      _resetPaintState();
                                      _handleBrushAdd(event.localPosition);
                                    } else if (isDeleting) {
                                      _resetPaintState();
                                      _handleBrushDelete(
                                        event.localPosition,
                                        pattern,
                                      );
                                    } else if (isZoom) {
                                      _lastZoomDragY = event.localPosition.dy;
                                    } else if (isSelecting) {
                                      setState(() {
                                        _selectionStart = event.localPosition;
                                        _selectionEnd = event.localPosition;
                                      });
                                    }
                                  },
                                  onPointerMove: (event) {
                                    if (isDrawing) {
                                      _handleBrushAdd(event.localPosition);
                                    } else if (isDeleting) {
                                      _handleBrushDelete(
                                        event.localPosition,
                                        pattern,
                                      );
                                    } else if (isZoom) {
                                      double deltaY =
                                          event.localPosition.dy -
                                          _lastZoomDragY;
                                      _lastZoomDragY = event.localPosition.dy;
                                      if (deltaY < 0) {
                                        _handleZoom(1 / 1.05);
                                      } else if (deltaY > 0) {
                                        _handleZoom(1.05);
                                      }
                                    } else if (isSelecting &&
                                        _selectionStart != null) {
                                      setState(() {
                                        _selectionEnd = event.localPosition;
                                      });

                                      _handleHorizontalAutoScroll(
                                        event.position.dx, // global X
                                      );

                                      // Also handle vertical auto-scroll for selection
                                      final screenSize = MediaQuery.of(
                                        context,
                                      ).size;
                                      _handleAutoScroll(
                                        event.position, // global Offset
                                        screenSize,
                                      );
                                    }
                                  },
                                  onPointerUp: (event) {
                                    _stopAutoScroll();
                                    if (isInteracting) {
                                      setState(() => isInteracting = false);
                                    }
                                    if (isDrawing) {
                                      _submitBrushAdd(pattern.id);
                                    } else if (isDeleting) {
                                      _submitBrushDelete(pattern.id);
                                    } else if (isSelecting) {
                                      _commitRangeSelection(pattern);
                                    }
                                  },
                                  child: GestureDetector(
                                    behavior: HitTestBehavior.translucent,
                                    child: MouseRegion(
                                      cursor: isDrawing
                                          ? SystemMouseCursors.copy
                                          : isPan
                                          ? SystemMouseCursors.move
                                          : isZoom
                                          ? SystemMouseCursors.resizeUpDown
                                          : SystemMouseCursors.basic,
                                      child: SizedBox(
                                        height: 128 * _keyHeight,
                                        width:
                                            pattern.lengthTicks * zoomX +
                                            1000, // Approx width
                                        child: ContextMenuWrapper(
                                          title: "Actions",
                                          actions: [
                                            DawContextAction(
                                              title: "Paste",
                                              icon: Icons.paste,
                                              onTap: () {
                                                if (_lastInteractionPos ==
                                                    null) {
                                                  return;
                                                }

                                                // Convert local pixel position to Ticks and MIDI Key
                                                int tick =
                                                    (_lastInteractionPos!.dx /
                                                            zoomX)
                                                        .round();
                                                int keyIndex =
                                                    (_lastInteractionPos!.dy /
                                                            _keyHeight)
                                                        .floor();
                                                int midiKey = (127 - keyIndex)
                                                    .clamp(0, 127);

                                                // Snap to grid for clean pasting
                                                int snap = _getSnapTicks(
                                                  gridDenom,
                                                );
                                                int snappedTick =
                                                    (tick / snap).round() *
                                                    snap;

                                                ref
                                                    .read(globalStateProvider)
                                                    .pasteNotesFromClipboardToPattern(
                                                      pattern.id,
                                                      snappedTick,
                                                      midiKey,
                                                    );
                                              },
                                            ),
                                          ],
                                          child: Stack(
                                            children: [
                                              // Grid background
                                              Positioned.fill(
                                                child: RepaintBoundary(
                                                  child: CustomPaint(
                                                    painter: _PianoGridPainter(
                                                      zoomX: zoomX,
                                                      keyHeight: _keyHeight,
                                                      gridDenom: gridDenom,
                                                    ),
                                                  ),
                                                ),
                                              ),

                                              // LAYER B1: Unselected Interactive Notes
                                              ...unselectedNotes.map((note) {
                                                final isPendingDelete =
                                                    _brushDeleteNoteIds
                                                        .contains(note.id);
                                                return _InteractiveNote(
                                                  key: ValueKey(note.id),
                                                  note: note,
                                                  noteId: note.id,
                                                  patternId: pattern.id,
                                                  generatorId:
                                                      widget.generatorId,
                                                  zoomX: zoomX,
                                                  keyHeight: _keyHeight,
                                                  selectedTool: selectedTool,
                                                  snapTicks: _getSnapTicks(
                                                    gridDenom,
                                                  ),
                                                  opacity: isPendingDelete
                                                      ? 0.3
                                                      : 0.8,
                                                  borderColor: Colors.white30,
                                                  onDragUpdate: (globalPos) {
                                                    if (selectedTool ==
                                                        PianoRollToolSelection
                                                            .grab) {
                                                      final screenSize =
                                                          MediaQuery.of(
                                                            context,
                                                          ).size;
                                                      _handleAutoScroll(
                                                        globalPos,
                                                        screenSize,
                                                      );
                                                      _handleHorizontalAutoScroll(
                                                        globalPos.dx,
                                                      );
                                                    }
                                                  },
                                                  onDragEnd: () {
                                                    _stopAutoScroll();
                                                  },
                                                  onTapOverride: () {
                                                    if (selectedTool ==
                                                            PianoRollToolSelection
                                                                .select ||
                                                        selectedTool ==
                                                            PianoRollToolSelection
                                                                .grab) {
                                                      // Selecting an unselected note clears previous selection and selects only this one
                                                      state.selectNotes({
                                                        note.id,
                                                      });
                                                    }
                                                  },
                                                );
                                              }),

                                              // LAYER B2: Selected Batch Notes
                                              if (selectedNotes.isNotEmpty)
                                                _InteractiveNoteGroup(
                                                  notes: selectedNotes,
                                                  patternId: pattern.id,
                                                  generatorId:
                                                      widget.generatorId,
                                                  zoomX: zoomX,
                                                  keyHeight: _keyHeight,
                                                  selectedTool: selectedTool,
                                                  snapTicks: _getSnapTicks(
                                                    gridDenom,
                                                  ),
                                                  pendingDeleteIds:
                                                      _brushDeleteNoteIds,
                                                  onDragUpdate: (globalPos) {
                                                    if (selectedTool ==
                                                        PianoRollToolSelection
                                                            .grab) {
                                                      final screenSize =
                                                          MediaQuery.of(
                                                            context,
                                                          ).size;
                                                      _handleAutoScroll(
                                                        globalPos,
                                                        screenSize,
                                                      );
                                                      _handleHorizontalAutoScroll(
                                                        globalPos.dx,
                                                      );
                                                    }
                                                  },
                                                  onDragEnd: () {
                                                    _stopAutoScroll();
                                                  },
                                                ),

                                              // LAYER C: Preview Add Notes
                                              ..._brushAddNotes.map((addInfo) {
                                                final (
                                                  key,
                                                  startTick,
                                                  duration,
                                                ) = addInfo;
                                                return Positioned(
                                                  top:
                                                      (127 - key) * _keyHeight +
                                                      1,
                                                  left: startTick * zoomX,
                                                  width: duration * zoomX < 5
                                                      ? 5
                                                      : duration * zoomX,
                                                  height: _keyHeight - 2,
                                                  child: IgnorePointer(
                                                    child: Container(
                                                      decoration: BoxDecoration(
                                                        color: Colors.pinkAccent
                                                            .withAlpha(128),
                                                        borderRadius:
                                                            BorderRadius.circular(
                                                              2,
                                                            ),
                                                        border: Border.all(
                                                          color: Colors.white54,
                                                        ),
                                                      ),
                                                    ),
                                                  ),
                                                );
                                              }),

                                              // LAYER D: Range Selection Box
                                              if (_selectionStart != null &&
                                                  _selectionEnd != null)
                                                Positioned.fromRect(
                                                  rect: Rect.fromPoints(
                                                    _selectionStart!,
                                                    _selectionEnd!,
                                                  ),
                                                  child: Container(
                                                    decoration: BoxDecoration(
                                                      color: Colors.blueAccent
                                                          .withAlpha(80),
                                                      border: Border.all(
                                                        color:
                                                            Colors.blueAccent,
                                                        width: 1.0,
                                                      ),
                                                    ),
                                                  ),
                                                ),
                                            ],
                                          ),
                                        ),
                                      ),
                                    ),
                                  ),
                                ),
                              ),
                            ),
                          ),
                          if (selectedNoteIds.isNotEmpty)
                            FloatingContextPanel(
                              actions: [
                                DawContextAction(
                                  title: "Delete",
                                  onTap: () {
                                    ref
                                        .read(globalStateProvider)
                                        .deletePatternNoteBatch(
                                          patternId: pattern.id,
                                          noteIds: selectedNoteIds.toList(),
                                        );
                                    ref
                                        .read(globalStateProvider)
                                        .clearNoteSelection();
                                  },
                                ),
                                DawContextAction(
                                  title: "Copy",
                                  onTap: () async {
                                    if (widget.patternId == null) {
                                      return;
                                    }
                                    await ref
                                        .read(globalStateProvider)
                                        .copyNotesFromPattern(
                                          widget.patternId!,
                                          selectedNoteIds.toList(),
                                        );
                                    // we do not clear because every DAW do this
                                  },
                                ),
                                DawContextAction(
                                  title: "Cut",
                                  onTap: () async {
                                    if (widget.patternId == null) {
                                      return;
                                    }

                                    await ref
                                        .read(globalStateProvider)
                                        .cutNotesFromPattern(
                                          widget.patternId!,
                                          selectedNoteIds.toList(),
                                        );
                                  },
                                ),
                              ],
                              onClose: () {
                                ref
                                    .read(globalStateProvider)
                                    .clearNoteSelection();
                              },
                              title: "${selectedNoteIds.length} Note(s)",
                            ),
                          Positioned.fill(
                            child: IgnorePointer(
                              ignoring: true,
                              child: PianoRollPlayheadOverlay(
                                scrollController: _gridHorizontalController,
                                zoomX: zoomX,
                                isInteracting: isInteracting,
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
                initialCenterNote: 72,
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _PianoRollToolbar extends ConsumerWidget {
  final int patternId;
  final String name;
  final VoidCallback onZoomIn;
  final VoidCallback onZoomOut;
  final GridSize gridDenom;
  final ValueChanged<GridSize?> onGridDenomChanged;

  const _PianoRollToolbar({
    required this.patternId,
    required this.name,
    required this.onZoomIn,
    required this.onZoomOut,
    required this.gridDenom,
    required this.onGridDenomChanged,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(globalStateProvider);
    final selectedTool = state.pianoRollTool;
    final generators = state.generators;
    final previewGeneratorId = state.previewGeneratorId;
    final isPatternPlaying = state.isPatternPlaying;
    final isPatternMode = state.isPatternMode;

    return Container(
      height: 50,
      color: Colors.grey.shade800,
      padding: const EdgeInsets.symmetric(horizontal: 10),
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: Row(
          children: [
            // Pattern transport
            StreamBuilder(
              stream: ref.read(globalStateProvider).positionStream,
              builder: (context, asyncSnapshot) {
                final feedback = asyncSnapshot.data;
                final isPatternPlaying =
                    feedback != null &&
                    feedback.isPlaying &&
                    feedback.isPatternMode;
                return IconButton(
                  icon: Icon(
                    isPatternPlaying ? Icons.stop : Icons.play_arrow,
                    color: isPatternPlaying ? Colors.orange : Colors.white70,
                  ),
                  onPressed: previewGeneratorId != null
                      ? () => _togglePatternPlayback(
                          previewGeneratorId,
                          patternId,
                        )
                      : null,
                  tooltip: isPatternPlaying ? 'Stop' : 'Play Pattern',
                  iconSize: 24,
                );
              },
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
              label: 'Grab',
              isActive: selectedTool == PianoRollToolSelection.grab,
              onTap: () =>
                  state.selectPianoRollTool(PianoRollToolSelection.grab),
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
            _ToolButton(
              icon: Icons.pan_tool,
              label: 'Pan',
              isActive: selectedTool == PianoRollToolSelection.pan,
              onTap: () =>
                  state.selectPianoRollTool(PianoRollToolSelection.pan),
            ),
            _ToolButton(
              icon: Icons.zoom_in,
              label: 'Zoom',
              isActive: selectedTool == PianoRollToolSelection.zoom,
              onTap: () =>
                  state.selectPianoRollTool(PianoRollToolSelection.zoom),
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
            DropdownButton<GridSize>(
              value: gridDenom,
              dropdownColor: Colors.grey.shade800,
              style: const TextStyle(color: Colors.white, fontSize: 12),
              underline: const SizedBox(),
              items: GridSize.values.map((element) {
                String label = "";
                switch (element) {
                  case GridSize.oneBar:
                    label = "4 Beats";
                    break;
                  case GridSize.twoBeat:
                    label = "2 Beats";
                    break;
                  case GridSize.full:
                    label = "1 Beat";
                    break;
                  case GridSize.half:
                    label = "1/2 Beat";
                    break;
                  case GridSize.third:
                    label = "1/3 Beat";
                    break;
                  case GridSize.quarter:
                    label = "1/4 Beat";
                    break;
                  case GridSize.sixth:
                    label = "1/6 Beat";
                    break;
                  case GridSize.eighth:
                    label = "1/8 Beat";
                  case GridSize.twelfth:
                    label = "1/12 Beat";
                    break;
                  case GridSize.sixteenth:
                    label = "1/16 Beat";
                    break;
                  case GridSize.thirtysecond:
                    label = "1/32 Beat";
                    break;
                  case GridSize.sixtyfourth:
                    label = "1/64 Beat";
                    break;
                  case GridSize.infinity:
                    label = "None";
                    break;
                }

                return DropdownMenuItem<GridSize>(
                  value: element, // Set the value to the enum itself
                  child: Text(label),
                );
              }).toList(),
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
                    child: Text(() {
                      final instanceType = entry.value.instanceType;
                      final name = switch (instanceType) {
                        UiGeneratorInstanceType_Plugin(:final field0) =>
                          field0.name,
                        _ => "Sampler",
                      };
                      return name;
                    }()),
                  ),
                ),
              ],
              onChanged: (value) {
                state.setPreviewGenerator(generatorId: value);

                if (value != null && isPatternPlaying && isPatternMode) {
                  try {
                    switchPatternGenerator(generatorId: value);
                  } catch (e) {
                    AppLogger.error("Failed to hot-swap generator: $e");
                  }
                }
              },
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildDivider() {
    return Container(width: 1, height: 30, color: Colors.grey.shade600);
  }

  void _togglePatternPlayback(int generatorId, int patternId) async {
    try {
      await togglePatternPlayback(
        patternId: patternId,
        generatorId: generatorId,
      );
    } catch (e) {
      AppLogger.error('Pattern playback error: $e');
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

class _InteractiveNote extends ConsumerStatefulWidget {
  final UiNote note;
  final int noteId;
  final int patternId;
  final int? generatorId; // Optional: To preview note while dragging
  final double zoomX;
  final double keyHeight;
  final PianoRollToolSelection selectedTool;
  final int snapTicks;
  final ValueChanged<Offset>? onDragUpdate;
  final VoidCallback? onDragEnd;
  final double opacity;
  final Color borderColor;
  final VoidCallback? onTapOverride;

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
    this.onDragUpdate,
    this.onDragEnd,
    this.opacity = 1.0,
    this.borderColor = Colors.white30,
    this.onTapOverride,
  });

  @override
  ConsumerState<_InteractiveNote> createState() {
    return _InteractiveNoteState();
  }
}

class _InteractiveNoteState extends ConsumerState<_InteractiveNote> {
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
        AppLogger.error(e.toString());
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
      child: Opacity(
        opacity: widget.opacity,
        child: MouseRegion(
          cursor: cursor,
          child: GestureDetector(
            behavior: HitTestBehavior.opaque,
            onTap: () {
              if (widget.onTapOverride != null) {
                widget.onTapOverride!();
                return;
              }

              if (widget.selectedTool == PianoRollToolSelection.delete) {
                final state = ref.read(globalStateProvider);
                final selectedIds = state.selectedNoteIds;
                if (selectedIds.length > 1 &&
                    selectedIds.contains(widget.noteId)) {
                  state.deletePatternNoteBatch(
                    patternId: widget.patternId,
                    noteIds: selectedIds.toList(),
                  );
                } else {
                  state.deletePatternNote(
                    patternId: widget.patternId,
                    noteId: widget.noteId,
                  );
                }
              }
            },
            onPanStart: (details) {
              if (widget.selectedTool != PianoRollToolSelection.grab) return;

              final renderBox = context.findRenderObject() as RenderBox;
              final localPos = renderBox.globalToLocal(details.globalPosition);

              // Hit test edge for resizing
              const edgeThreshold = 3.0;

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
              widget.onDragUpdate?.call(details.globalPosition);
            },
            onPanEnd: (details) {
              if (_currentPreviewKey != null) {
                _playNote(_currentPreviewKey!, false);
                _currentPreviewKey = null;
              }

              final state = ref.read(globalStateProvider);

              if (widget.generatorId != null) {
                try {
                  playPreviewNoteGenerator(
                    generatorId: widget.generatorId!,
                    noteKey: widget.note.key,
                    velocity: 100,
                    isOn: false,
                  );
                } catch (e) {
                  AppLogger.error(e.toString());
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

                final selectedIds = state.selectedNoteIds;
                if (selectedIds.length > 1 &&
                    selectedIds.contains(widget.noteId)) {
                  final tickDelta = newStartTick - widget.note.startTick;
                  final keyDelta = newKey - widget.note.key;

                  final pattern = state.patterns[widget.patternId];
                  if (pattern != null) {
                    final updates = <(int, int, int)>[];
                    for (final n in pattern.notes) {
                      if (selectedIds.contains(n.id)) {
                        int targetStartTick = n.startTick + tickDelta;
                        if (targetStartTick < 0) targetStartTick = 0;
                        int targetKey = (n.key + keyDelta).clamp(0, 127);
                        updates.add((n.id, targetStartTick, targetKey));
                      }
                    }
                    state.movePatternNoteBatch(
                      patternId: widget.patternId,
                      updates: updates,
                    );
                  }
                } else {
                  state.movePatternNote(
                    patternId: widget.patternId,
                    noteId: widget.noteId,
                    newStartTick: newStartTick,
                    newKey: newKey,
                  );
                }
              } else if (_mode == _NoteDragMode.resizeRight) {
                int rawDuration = (_localWidth / widget.zoomX).round();
                int newDuration = (rawDuration / snap).round() * snap;
                if (newDuration < 10) newDuration = snap;

                final selectedIds = state.selectedNoteIds;
                if (selectedIds.length > 1 &&
                    selectedIds.contains(widget.noteId)) {
                  final durationDelta = newDuration - widget.note.duration;

                  final pattern = state.patterns[widget.patternId];
                  if (pattern != null) {
                    final updates = <(int, int)>[];
                    for (final n in pattern.notes) {
                      if (selectedIds.contains(n.id)) {
                        int targetDuration = n.duration + durationDelta;
                        if (targetDuration < 10) targetDuration = 10;
                        updates.add((n.id, targetDuration));
                      }
                    }
                    state.resizePatternNoteBatch(
                      patternId: widget.patternId,
                      updates: updates,
                    );
                  }
                } else {
                  state.resizePatternNote(
                    patternId: widget.patternId,
                    noteId: widget.noteId,
                    newDuration: newDuration,
                  );
                }
              }
              setState(() {
                _mode = _NoteDragMode.none;
                // Snap visual state immediately to grid so it looks clean
                // _localTop =
                //     (_localTop / widget.keyHeight).round() * widget.keyHeight;
              });
              widget.onDragEnd?.call();
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
      ),
    );
  }
}

class _PianoGridPainter extends CustomPainter {
  final double zoomX;
  final double keyHeight;
  final GridSize gridDenom;

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
    double ticksPerGrid = 960.0 * 4.0 / gridDenom.value;
    double pixelsPerGrid = ticksPerGrid * zoomX;

    if (pixelsPerGrid < 4) return;

    double currentX = 0;
    int gridIndex = 0;

    while (currentX < size.width) {
      // Calculate absolute ticks for precise modulo math
      int currentTick = (gridIndex * ticksPerGrid).round();

      // 3840 ticks = 1 Bar (4/4 time). 960 ticks = 1 Beat.
      bool isBar = (currentTick % 3840) == 0;
      bool isBeat = (currentTick % 960) == 0;

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

/// A specialized widget to handle grouped interactions for multiple selected notes.
/// It creates a relative coordinate system bound box and accurately applies deltas to the whole group.
class _InteractiveNoteGroup extends ConsumerStatefulWidget {
  final List<UiNote> notes;
  final int patternId;
  final int? generatorId;
  final double zoomX;
  final double keyHeight;
  final PianoRollToolSelection selectedTool;
  final int snapTicks;
  final Set<int> pendingDeleteIds;
  final ValueChanged<Offset>? onDragUpdate;
  final VoidCallback? onDragEnd;

  const _InteractiveNoteGroup({
    required this.notes,
    required this.patternId,
    this.generatorId,
    required this.zoomX,
    required this.keyHeight,
    required this.selectedTool,
    required this.snapTicks,
    required this.pendingDeleteIds,
    this.onDragUpdate,
    this.onDragEnd,
  });

  @override
  ConsumerState<_InteractiveNoteGroup> createState() =>
      _InteractiveNoteGroupState();
}

class _InteractiveNoteGroupState extends ConsumerState<_InteractiveNoteGroup> {
  double _dragOffsetX = 0;
  double _dragOffsetY = 0;
  double _dragWidthDelta = 0;
  _NoteDragMode _mode = _NoteDragMode.none;

  int? _currentPreviewKey;

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
        AppLogger.error(e.toString());
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    if (widget.notes.isEmpty) return const SizedBox.shrink();

    int minTick = widget.notes.map((n) => n.startTick).reduce(min);
    int maxTick = widget.notes.map((n) => n.startTick + n.duration).reduce(max);
    int maxKey = widget.notes.map((n) => n.key).reduce(max);
    int minKey = widget.notes.map((n) => n.key).reduce(min);

    double baseLeft = minTick * widget.zoomX;
    double baseTop = (127 - maxKey) * widget.keyHeight;
    double baseWidth = (maxTick - minTick) * widget.zoomX;
    double baseHeight = (maxKey - minKey + 1) * widget.keyHeight;

    MouseCursor cursor = SystemMouseCursors.click;
    if (widget.selectedTool == PianoRollToolSelection.delete) {
      cursor = SystemMouseCursors.basic;
    } else if (_mode != _NoteDragMode.none) {
      cursor = SystemMouseCursors.grabbing;
    } else {
      cursor = SystemMouseCursors.click;
    }

    return Positioned(
      left: baseLeft + _dragOffsetX,
      top: baseTop + _dragOffsetY,
      width: max(0, baseWidth + _dragWidthDelta),
      height: baseHeight,
      child: MouseRegion(
        cursor: cursor,
        child: Stack(
          clipBehavior: Clip.none,
          children: widget.notes.map((note) {
            double noteLeft = (note.startTick - minTick) * widget.zoomX;
            double noteTop = (maxKey - note.key) * widget.keyHeight;
            double noteWidth = note.duration * widget.zoomX;

            if (_mode == _NoteDragMode.resizeRight) {
              noteWidth += _dragWidthDelta;
            }
            if (noteWidth < 5) noteWidth = 5;

            final isPendingDelete = widget.pendingDeleteIds.contains(note.id);

            return Positioned(
              left: noteLeft,
              top: noteTop - 1,
              width: noteWidth,
              height: widget.keyHeight - 2,
              child: Opacity(
                opacity: isPendingDelete ? 0.3 : 1.0,
                child: GestureDetector(
                  behavior: HitTestBehavior.opaque,
                  onTap: () {
                    if (widget.selectedTool == PianoRollToolSelection.delete) {
                      ref
                          .read(globalStateProvider)
                          .deletePatternNoteBatch(
                            patternId: widget.patternId,
                            noteIds: widget.notes.map((e) => e.id).toList(),
                          );
                      ref.read(globalStateProvider).clearNoteSelection();
                    }
                  },
                  onPanStart: (details) {
                    if (widget.selectedTool != PianoRollToolSelection.grab) {
                      return;
                    }
                    final renderBox = context.findRenderObject() as RenderBox;
                    final localPos = renderBox.globalToLocal(
                      details.globalPosition,
                    );
                    const edgeThreshold = 10.0;

                    // localPos is relative to the Bounding Box Stack.
                    // Verify if the grab hits the right-side edge of THIS specific child note.
                    if (localPos.dx > noteLeft + noteWidth - edgeThreshold &&
                        localPos.dx <= noteLeft + noteWidth) {
                      setState(() => _mode = _NoteDragMode.resizeRight);
                    } else {
                      setState(() => _mode = _NoteDragMode.move);
                    }

                    _currentPreviewKey = note.key;
                    _playNote(_currentPreviewKey!, true);
                  },
                  onPanUpdate: (details) {
                    if (_mode == _NoteDragMode.none) return;
                    setState(() {
                      if (_mode == _NoteDragMode.move) {
                        _dragOffsetX += details.delta.dx;
                        _dragOffsetY += details.delta.dy;
                      } else if (_mode == _NoteDragMode.resizeRight) {
                        _dragWidthDelta += details.delta.dx;
                      }
                    });
                    widget.onDragUpdate?.call(details.globalPosition);
                  },

                  onPanEnd: (details) {
                    if (_currentPreviewKey != null) {
                      _playNote(_currentPreviewKey!, false);
                      _currentPreviewKey = null;
                    }

                    final state = ref.read(globalStateProvider);
                    int snap = widget.snapTicks;

                    if (_mode == _NoteDragMode.move) {
                      int rawTickDelta = (_dragOffsetX / widget.zoomX).round();
                      int snappedTickDelta =
                          (rawTickDelta / snap).round() * snap;

                      // Negative Y means moving UP to a higher pitch
                      int keyDelta = -(_dragOffsetY / widget.keyHeight).round();

                      if (snappedTickDelta != 0 || keyDelta != 0) {
                        final updates = <(int, int, int)>[];
                        for (final n in widget.notes) {
                          int targetStartTick = n.startTick + snappedTickDelta;
                          if (targetStartTick < 0) targetStartTick = 0;
                          int targetKey = (n.key + keyDelta).clamp(0, 127);
                          updates.add((n.id, targetStartTick, targetKey));
                        }
                        state.movePatternNoteBatch(
                          patternId: widget.patternId,
                          updates: updates,
                        );
                      }
                    } else if (_mode == _NoteDragMode.resizeRight) {
                      int rawDurationDelta = (_dragWidthDelta / widget.zoomX)
                          .round();
                      int snappedDurationDelta =
                          (rawDurationDelta / snap).round() * snap;

                      if (snappedDurationDelta != 0) {
                        final updates = <(int, int)>[];
                        for (final n in widget.notes) {
                          int targetDuration =
                              n.duration + snappedDurationDelta;
                          if (targetDuration < 10) targetDuration = 10;
                          updates.add((n.id, targetDuration));
                        }
                        state.resizePatternNoteBatch(
                          patternId: widget.patternId,
                          updates: updates,
                        );
                      }
                    }

                    setState(() {
                      _mode = _NoteDragMode.none;
                      _dragOffsetX = 0;
                      _dragOffsetY = 0;
                      _dragWidthDelta = 0;
                    });
                    widget.onDragEnd?.call();
                  },
                  child: Container(
                    decoration: BoxDecoration(
                      color: _mode != _NoteDragMode.none
                          ? Colors.pink
                          : Colors.pinkAccent,
                      borderRadius: BorderRadius.circular(2),
                      border: Border.all(
                        color: Colors.white,
                        width: 1.5,
                      ), // Thick white border for selection
                    ),
                    child: noteWidth > 30
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
          }).toList(),
        ),
      ),
    );
  }
}

class _PianoRollPlayheadPainter extends CustomPainter {
  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      // Using orange to match your pattern play button theme
      ..color = Colors.orangeAccent
      ..style = PaintingStyle.fill;

    final path = Path();
    path.moveTo(0, 0); // Top Left
    path.lineTo(size.width, 0); // Top Right
    path.lineTo(size.width / 2, size.height); // Bottom Center
    path.close();

    canvas.drawPath(path, paint);
    canvas.drawShadow(path, Colors.black, 2.0, false);
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}

class PianoRollPlayheadOverlay extends ConsumerStatefulWidget {
  final ScrollController scrollController;
  final double zoomX; // Pixels per tick
  final bool isInteracting;

  const PianoRollPlayheadOverlay({
    super.key,
    required this.scrollController,
    required this.zoomX,
    this.isInteracting = false,
  });

  @override
  ConsumerState<PianoRollPlayheadOverlay> createState() =>
      _PianoRollPlayheadOverlayState();
}

class _PianoRollPlayheadOverlayState
    extends ConsumerState<PianoRollPlayheadOverlay> {
  late Stream<UiTransportFeedback> _positionStream;
  int _lastKnownTicks = 0;

  @override
  void initState() {
    super.initState();
    _positionStream = ref.read(globalStateProvider).positionStream;
  }

  int _getTicksFromFeedback(UiTransportFeedback pos) {
    if (!pos.isPatternMode || pos.sampleRate <= 0) return 0;

    // Convert samples directly to ticks safely
    const ppq = 960;
    final ticks =
        (pos.patternSamples * ppq * pos.tempo) / (60.0 * pos.sampleRate);
    return ticks.round();
  }

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final viewportWidth = constraints.maxWidth;

        return StreamBuilder<UiTransportFeedback>(
          stream: _positionStream,
          builder: (context, snapshot) {
            if (snapshot.hasData) {
              _lastKnownTicks = _getTicksFromFeedback(snapshot.data!);
            }

            // Calculate exact pixel position on the internal grid
            final playheadAbsoluteX = _lastKnownTicks * widget.zoomX;

            return AnimatedBuilder(
              animation: widget.scrollController,
              builder: (context, child) {
                double scrollOffset = 0;
                if (widget.scrollController.hasClients) {
                  scrollOffset = widget.scrollController.offset;
                }

                // Map grid position to screen viewport position
                final double left = playheadAbsoluteX - scrollOffset;

                // Optimization: Cull rendering if off-screen to save CPU
                if (left < -20 || left > viewportWidth + 20) {
                  return const SizedBox.shrink();
                }

                return Stack(
                  clipBehavior: Clip.none,
                  children: [
                    Positioned(
                      left: left - 7.5, // Center the 15px wide playhead
                      top: 0,
                      bottom: 0,
                      width: 15,
                      child: Column(
                        children: [
                          SizedBox(
                            height: 10,
                            width: 15,
                            child: CustomPaint(
                              painter: _PianoRollPlayheadPainter(),
                            ),
                          ),
                          Expanded(
                            child: Container(
                              width: 1.5,
                              color: Colors.orangeAccent.withAlpha(
                                widget.isInteracting ? 100 : 204,
                              ),
                            ),
                          ),
                        ],
                      ),
                    ),
                  ],
                );
              },
            );
          },
        );
      },
    );
  }
}
