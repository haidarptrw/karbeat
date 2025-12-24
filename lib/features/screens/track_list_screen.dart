import 'dart:async';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:karbeat/features/playlist/playhead.dart';
import 'package:karbeat/features/playlist/track_slot.dart';
import 'package:karbeat/src/rust/api/project.dart';
import 'package:karbeat/src/rust/core/project.dart';
import 'package:karbeat/state/app_state.dart';
import 'package:karbeat/utils/scroll_behavior.dart';
import 'package:linked_scroll_controller/linked_scroll_controller.dart';
import 'package:provider/provider.dart';

class TrackListScreen extends StatelessWidget {
  const TrackListScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final parentHeight = constraints.maxHeight;
        if (parentHeight.isInfinite) return const SizedBox();

        final calculatedHeight = parentHeight * 0.20;
        final double itemHeight = calculatedHeight.clamp(80.0, 150.0);
        const double headerWidth = 220.0;

        return Consumer<KarbeatState>(
          builder: (context, state, child) {
            final tracks = state.tracks.values.toList()
              ..sort((a, b) => a.id.compareTo(b.id));

            return _SplitTrackView(
              tracks: tracks,
              itemHeight: itemHeight,
              headerWidth: headerWidth,
            );
          },
        );
      },
    );
  }
}

class _SplitTrackView extends StatefulWidget {
  final List<UiTrack> tracks;
  final double itemHeight;
  final double headerWidth;

  const _SplitTrackView({
    required this.tracks,
    required this.itemHeight,
    required this.headerWidth,
  });

  @override
  State<_SplitTrackView> createState() => _SplitTrackViewState();
}

class _SplitTrackViewState extends State<_SplitTrackView> {
  late LinkedScrollControllerGroup _verticalControllers;
  late ScrollController _headerController;
  late ScrollController _timelineController;

  // Horizontal Scrolling (Ruler <-> Tracks)
  late LinkedScrollControllerGroup _horizontalControllers;
  late ScrollController _rulerController; // Controller 1: Top Ruler
  late ScrollController _trackContentController; // Controller 2: Bottom Content

  // Local state for ghost clip
  Offset? _mousePos;

  // LocalState for width
  double _timelineWidth = 2000.0;

  int _activeSampleRate = 44100;
  StreamSubscription? _posSub;

  bool _isCtrlPressed = false;

  @override
  void initState() {
    super.initState();
    // Initialize the Linked Group
    _verticalControllers = LinkedScrollControllerGroup();
    _headerController = _verticalControllers.addAndGet();
    _timelineController = _verticalControllers.addAndGet();
    _horizontalControllers = LinkedScrollControllerGroup();
    _rulerController = _horizontalControllers.addAndGet();
    _trackContentController = _horizontalControllers.addAndGet();
    _trackContentController.addListener(_handleScrollExpansion);
    HardwareKeyboard.instance.addHandler(_handleKeyEvents);

    final state = context.read<KarbeatState>();
    _activeSampleRate = state.hardwareConfig.sampleRate > 0
        ? state.hardwareConfig.sampleRate
        : 44100;

    // FIX: Listen to the stream to detect the REAL negotiated sample rate (e.g. 48000)
    // The engine sends this in every position update.
    _posSub = state.positionStream.listen((pos) {
      if (!mounted) return;
      if (pos.sampleRate > 0 && pos.sampleRate != _activeSampleRate) {
        // Only setState if it changed to avoid spamming rebuilds
        setState(() {
          _activeSampleRate = pos.sampleRate;
        });
      }
    });
  }

  @override
  void dispose() {
    _trackContentController.removeListener(_handleScrollExpansion);
    _headerController.dispose();
    _timelineController.dispose();
    _rulerController.dispose();
    _trackContentController.dispose();
    super.dispose();
  }

  bool _handleKeyEvents(KeyEvent event) {
    final isCtrl =
        HardwareKeyboard.instance.logicalKeysPressed.contains(
          LogicalKeyboardKey.controlLeft,
        ) ||
        HardwareKeyboard.instance.logicalKeysPressed.contains(
          LogicalKeyboardKey.controlRight,
        );

    if (isCtrl != _isCtrlPressed) {
      // Check mounted before setState in case of fast dispose
      if (mounted) {
        setState(() {
          _isCtrlPressed = isCtrl;
        });
      }
    }
    return false;
  }

  void _handleScrollExpansion() {
    // If the user scrolls within 500px of the edge...
    final maxScroll = _trackContentController.position.maxScrollExtent;
    final currentScroll = _trackContentController.offset;

    if (currentScroll >= maxScroll - 500) {
      // ... Add more space (e.g., another 2000px)
      setState(() {
        _timelineWidth += 2000.0;
      });
    }
  }

  void _updateZoom(double newZoom) {
    // Define min/max zoom limits to prevent bugs
    final clamped = newZoom.clamp(0.01, 5000.0);

    // Assuming you have a setter in KarbeatState.
    // If not, add: void setHorizontalZoom(double val) { horizontalZoomLevel = val; notifyListeners(); }
    context.read<KarbeatState>().setHorizontalZoom(clamped);
  }

  void _handleTimelineGesture(
    BuildContext context,
    Offset localPosition, {
    bool isDrag = false,
  }) {
    final state = context.read<KarbeatState>();
    double scrollX = 0;
    if (_trackContentController.hasClients) {
      scrollX = _trackContentController.offset;
    }
    final double absoluteX = localPosition.dx + scrollX;
    if (absoluteX < 0) return;

    final zoomLevel = state.horizontalZoomLevel;

    switch (state.selectedTool) {
      case ToolSelection.scrub:
        final double samples = absoluteX * zoomLevel;
        state.seekTo(samples.toInt());
        break;
      case ToolSelection.zoom:
        break;
      case ToolSelection.draw:
        setState(() {
          _mousePos = localPosition;
        });
        _updatePlacementTarget(state);
        break;
      case ToolSelection.pointer:
      case ToolSelection.cut:
      default:
        break;
    }
  }

  @override
  Widget build(BuildContext context) {
    // Calculate total height to ensure both lists have exactly same extent
    // +1 for the Add Button row
    final int itemCount = widget.tracks.length + 1;
    final state = context.read<KarbeatState>();
    final isPlacing = context.select<KarbeatState, bool>((s) => s.isPlacing);
    final selectedTool = context.select<KarbeatState, ToolSelection>(
      (s) => s.selectedTool,
    );
    final currentTimelineWidth = _timelineWidth;

    return Stack(
      children: [
        Row(
          children: [
            // --- LEFT: TRACK HEADERS ---
            SizedBox(
              width: widget.headerWidth,
              child: Column(
                children: [
                  // Optional: Fixed Header Row (e.g. "Name", "Mute")
                  Container(
                    height: 30,
                    color: Colors.grey.shade800,
                    alignment: Alignment.centerLeft,
                    padding: const EdgeInsets.only(left: 10),
                    child: const Text(
                      "Tracks",
                      style: TextStyle(color: Colors.white70, fontSize: 12),
                    ),
                  ),
                  Expanded(
                    child: ListView.builder(
                      controller: _headerController, // Controller 1
                      padding: EdgeInsets.zero,
                      itemCount: itemCount,
                      itemBuilder: (context, index) {
                        if (index == widget.tracks.length) {
                          return _buildAddButton();
                        }
                        return _buildTrackHeader(widget.tracks[index]);
                      },
                    ),
                  ),
                ],
              ),
            ),

            // --- DIVIDER ---
            Container(width: 1, color: Colors.black),

            // --- RIGHT: TIMELINE ---
            Expanded(
              child: Listener(
                onPointerSignal: (event) {
                  if (event is PointerScrollEvent) {
                    if (_isCtrlPressed) {
                      final currentZoom = context
                          .read<KarbeatState>()
                          .horizontalZoomLevel;
                      final double multiplier = event.scrollDelta.dy > 0
                          ? 0.9
                          : 1.1;
                      _updateZoom(currentZoom * multiplier);
                    }
                  }
                },
                child: GestureDetector(
                  behavior: HitTestBehavior.translucent,
                  onTapDown: (details) => _handleTimelineGesture(
                    context,
                    details.localPosition,
                    isDrag: false,
                  ),
                  onPanUpdate: (details) {
                    if (selectedTool == ToolSelection.zoom) {
                      final currentZoom = state.horizontalZoomLevel;
                      double multiplier = 1.0 - (details.delta.dy * 0.01);
                      _updateZoom(currentZoom * multiplier);
                      return;
                    }
                    if (selectedTool == ToolSelection.scrub) {
                      _handleTimelineGesture(
                        context,
                        details.localPosition,
                        isDrag: true,
                      );
                      return;
                    }
                    if (selectedTool == ToolSelection.draw || isPlacing) {
                      setState(() => _mousePos = details.localPosition);
                      _updatePlacementTarget(state);
                    }
                  },
                  child: Column(
                    children: [
                      // Optional: Time Ruler Header (Horizontal Scrollable)
                      // We would need another sync controller for the ruler + body horizontal scroll.
                      GestureDetector(
                        onTapDown: (details) {
                          double scrollX = _rulerController.hasClients
                              ? _rulerController.offset
                              : 0;
                          double absoluteX = details.localPosition.dx + scrollX;
                          final samples = absoluteX * state.horizontalZoomLevel;
                          state.seekTo(samples.toInt());
                        },
                        onPanUpdate: (details) {
                          double scrollX = _rulerController.hasClients
                              ? _rulerController.offset
                              : 0;
                          double absoluteX = details.localPosition.dx + scrollX;
                          final samples = absoluteX * state.horizontalZoomLevel;
                          state.seekTo(samples.toInt());
                        },
                        child: Container(
                          height: 30,
                          color: Colors.grey.shade800,
                          width: double.infinity,
                          child: SingleChildScrollView(
                            scrollDirection: Axis.horizontal,
                            controller: _rulerController,
                            physics: _isCtrlPressed
                                ? const NeverScrollableScrollPhysics()
                                : const ClampingScrollPhysics(),
                            child: SizedBox(
                              width:
                                  currentTimelineWidth, // Matches track list width
                              height: 30,
                              child: _TimelineRuler(
                                scrollController: _rulerController,
                                sampleRate: _activeSampleRate,
                              ),
                            ),
                          ),
                        ),
                      ),
                      Expanded(
                        child: MouseRegion(
                          cursor: isPlacing
                              ? SystemMouseCursors.move
                              : SystemMouseCursors.basic,
                          onHover: null,
                          child: GestureDetector(
                            behavior: HitTestBehavior.translucent,
                            onPanUpdate: null,
                            onTapDown: isPlacing
                                ? (details) {
                                    setState(() {
                                      _mousePos = details.localPosition;
                                    });
                                    _updatePlacementTarget(state);
                                  }
                                : null,
                            child: ScrollConfiguration(
                              behavior: DragScrollBehavior(),
                              child: SingleChildScrollView(
                                scrollDirection: Axis.horizontal,
                                controller: _trackContentController,
                                // Physics to match desktop feel
                                physics: _isCtrlPressed
                                    ? const NeverScrollableScrollPhysics()
                                    : const ClampingScrollPhysics(),
                                child: SizedBox(
                                  width: currentTimelineWidth,
                                  child: ListView.builder(
                                    controller:
                                        _timelineController, // Controller 2 (Synced Vertically)
                                    padding: EdgeInsets.zero,
                                    itemCount: itemCount,
                                    itemBuilder: (context, index) {
                                      if (index == widget.tracks.length) {
                                        // Empty space matching Add Button height
                                        return SizedBox(height: 60);
                                      }
                                      return IgnorePointer(
                                        ignoring: isPlacing,
                                        child: KarbeatTrackSlot(
                                          trackId: widget.tracks[index].id,
                                          height: widget.itemHeight,
                                          horizontalScrollController:
                                              _trackContentController,
                                          sampleRate: _activeSampleRate,
                                        ),
                                      );
                                    },
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
              ),
            ),
          ],
        ),
        if (isPlacing && _mousePos != null) _buildGhostClip(context),
        Positioned.fill(
          child: IgnorePointer(
            ignoring: false,
            child: TimelinePlayheadSeeker(
              headerWidth: widget.headerWidth,
              scrollController: _trackContentController,
              onSeek: (int newSamples) {
                // Clamp to 0
                final safeSamples = newSamples < 0 ? 0 : newSamples;

                // Call your state to update position
                // Assuming you have a method like this:
                context.read<KarbeatState>().seekTo(safeSamples);

                print("Seeking to: $safeSamples samples");
              },
            ),
          ),
        ),
        if (isPlacing)
          Positioned(
            bottom: 30,
            right: 30,
            child: Row(
              children: [
                FloatingActionButton.extended(
                  heroTag: 'cancel_place',
                  label: const Text("Cancel"),
                  icon: const Icon(Icons.close),
                  backgroundColor: Colors.redAccent,
                  onPressed: () => state.cancelPlacement(),
                ),
                const SizedBox(width: 16),
                FloatingActionButton.extended(
                  onPressed: () => state.confirmPlacement(),
                  label: const Text('Confirm'),
                  heroTag: 'confirm_place',
                  icon: Icon(Icons.check),
                  backgroundColor: Colors.greenAccent,
                ),
              ],
            ),
          ),
      ],
    );
  }

  void _updatePlacementTarget(KarbeatState state) {
    if (_mousePos == null) return;

    // 1. Calculate Absolute Y (Mouse + Scroll)
    double scrollY = 0;
    if (_timelineController.hasClients) {
      scrollY = _timelineController.offset;
    }
    double absoluteY = _mousePos!.dy + scrollY;

    // 2. Determine Track Index
    int trackIndex = (absoluteY / widget.itemHeight).floor();
    trackIndex = trackIndex.clamp(0, widget.tracks.length - 1);
    final targetTrack = widget.tracks[trackIndex];

    // 3. Calculate Absolute X (Mouse + Scroll)
    double scrollX = 0;
    if (_trackContentController.hasClients) {
      scrollX = _trackContentController.offset;
    }
    double absoluteX = _mousePos!.dx + scrollX;

    if (absoluteX < 0) absoluteX = 0;

    // 4. Convert X Pixels -> Samples
    final zoomLevel = context.read<KarbeatState>().horizontalZoomLevel;
    double samples = absoluteX * zoomLevel;

    // 5. Update State
    state.updatePlacementTarget(targetTrack.id, samples);
  }

  Widget _buildGhostClip(BuildContext context) {
    // We map the absolute coordinates back to screen coordinates
    // This is essentially just drawing where the mouse is, but snapped to rows

    // We need logic to snap the ghost Y to the row, but let X float
    // 1. Get current Scroll Offset Y to align grid
    double scrollY = 0;
    if (_timelineController.hasClients) {
      scrollY = _timelineController.offset;
    }
    double absoluteY = _mousePos!.dy + scrollY;
    int trackIndex = (absoluteY / widget.itemHeight).floor();
    trackIndex = trackIndex.clamp(0, widget.tracks.length - 1);

    double top = (trackIndex * widget.itemHeight) - scrollY;

    // Offset by header height (approx) + Header Row
    top += 30;

    // Left position is just mouse X offset by header width
    double left = widget.headerWidth + _mousePos!.dx;

    // Safety check to keep it in timeline area
    if (left < widget.headerWidth) left = widget.headerWidth;

    return Positioned(
      left: left,
      top: top,
      width: 150, // Preview width
      height: widget.itemHeight - 4,
      child: GestureDetector(
        // ENABLE Dragging on the ghost itself
        onPanUpdate: (details) {
          setState(() {
            // Update _mousePos relative to the drag delta
            if (_mousePos != null) {
              _mousePos = _mousePos! + details.delta;
            }
          });
          // Update the logic state
          final state = context.read<KarbeatState>();
          _updatePlacementTarget(state);
        },
        child: Opacity(
          opacity: 0.7,
          // REMOVE IgnorePointer so it can catch the Drag events
          child: MouseRegion(
            cursor: SystemMouseCursors.move, // Indicate draggable
            child: Container(
              decoration: BoxDecoration(
                color: Colors.cyanAccent.withAlpha(100),
                border: Border.all(color: Colors.cyanAccent, width: 2),
                borderRadius: BorderRadius.circular(4),
              ),
              child: const Center(
                child: Text(
                  "Place Here",
                  style: TextStyle(
                    color: Colors.white,
                    fontWeight: FontWeight.bold,
                    shadows: [Shadow(color: Colors.black, blurRadius: 2)],
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildAddButton() {
    return SizedBox(
      height: 60,
      child: Center(
        child: TextButton.icon(
          onPressed: () => _showAddTrackDialog(context),
          icon: const Icon(Icons.add, color: Colors.white54),
          label: const Text(
            "Add New Track",
            style: TextStyle(color: Colors.white54),
          ),
        ),
      ),
    );
  }

  Widget _buildTrackHeader(UiTrack track) {
    return SizedBox(
      height: widget.itemHeight,
      child: Container(
        margin: const EdgeInsets.only(bottom: 2),
        padding: const EdgeInsets.symmetric(horizontal: 10),
        decoration: BoxDecoration(
          color: Colors.grey.shade300,
          border: Border(
            bottom: BorderSide(color: Colors.grey.shade400, width: 1),
            right: BorderSide(color: Colors.grey.shade400, width: 1),
          ),
        ),
        child: Row(
          children: [
            Icon(_getTrackIcon(track.trackType), color: Colors.grey.shade700),
            const SizedBox(width: 10),
            Expanded(
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    track.name,
                    style: TextStyle(
                      color: Colors.grey.shade800,
                      fontWeight: FontWeight.bold,
                      fontSize: 14,
                    ),
                    overflow: TextOverflow.ellipsis,
                  ),
                  Text(
                    "ID: ${track.id} | ${track.trackType.name.toUpperCase()}",
                    style: TextStyle(color: Colors.grey.shade600, fontSize: 10),
                  ),
                ],
              ),
            ),
            Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                InkWell(
                  onTap: () {},
                  child: const Icon(
                    Icons.mic_off,
                    size: 16,
                    color: Colors.grey,
                  ),
                ),
                const SizedBox(height: 4),
                InkWell(
                  onTap: () {},
                  child: const Icon(
                    Icons.volume_up,
                    size: 16,
                    color: Colors.grey,
                  ),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  IconData _getTrackIcon(TrackType type) {
    switch (type) {
      case TrackType.audio:
        return Icons.graphic_eq;
      case TrackType.midi:
        return Icons.piano;
      case TrackType.automation:
        return Icons.show_chart;
    }
  }

  void _showAddTrackDialog(BuildContext context) {
    // Access the list from state
    final availablePlugins = context.read<KarbeatState>().availableGenerators;

    showDialog(
      context: context,
      builder: (ctx) => SimpleDialog(
        title: const Text("Add New Track"),
        children: [
          SimpleDialogOption(
            onPressed: () {
              Navigator.pop(ctx);
              context.read<KarbeatState>().addTrack(TrackType.audio);
            },
            child: const Row(
              children: [
                Icon(Icons.graphic_eq, color: Colors.cyanAccent),
                SizedBox(width: 10),
                Text("Audio Track"),
              ],
            ),
          ),
          const Divider(),
          const Padding(
            padding: EdgeInsets.symmetric(horizontal: 24, vertical: 8),
            child: Text(
              "Instruments",
              style: TextStyle(color: Colors.grey, fontSize: 12),
            ),
          ),

          // DYNAMICALLY GENERATE OPTIONS
          if (availablePlugins.isEmpty)
            const Padding(
              padding: EdgeInsets.symmetric(horizontal: 24),
              child: Text(
                "No plugins found",
                style: TextStyle(color: Colors.grey),
              ),
            )
          else
            ...availablePlugins.map(
              (name) => _buildGeneratorOption(ctx, name, Icons.piano),
            ),
        ],
      ),
    );
  }

  Widget _buildGeneratorOption(BuildContext ctx, String name, IconData icon) {
    return SimpleDialogOption(
      onPressed: () {
        Navigator.pop(ctx);
        context.read<KarbeatState>().addMidiTrackWithGenerator(name);
      },
      child: Row(
        children: [
          Icon(icon, color: Colors.orangeAccent),
          const SizedBox(width: 10),
          Text(name),
        ],
      ),
    );
  }
}

class _TimelineRuler extends StatelessWidget {
  final ScrollController scrollController;
  final int sampleRate;

  const _TimelineRuler({
    required this.scrollController,
    required this.sampleRate,
  });

  @override
  Widget build(BuildContext context) {
    // Read state for drawing
    final zoomLevel = context.select<KarbeatState, double>(
      (s) => s.horizontalZoomLevel,
    );
    final tempo = context.select<KarbeatState, double>((s) => s.tempo);
    final safeSampleRate = sampleRate <= 0 ? 48000 : sampleRate;

    return RepaintBoundary(
      child: CustomPaint(
        size: Size.zero,
        painter: _TimelineRulerPainter(
          zoomLevel: zoomLevel,
          tempo: tempo,
          sampleRate: safeSampleRate,
          scrollController: scrollController,
        ),
      ),
    );
  }
}

class _TimelineRulerPainter extends CustomPainter {
  final double zoomLevel;
  final double tempo;
  final int sampleRate;
  final ScrollController scrollController;

  _TimelineRulerPainter({
    required this.zoomLevel,
    required this.tempo,
    required this.sampleRate,
    required this.scrollController,
  }) : super(repaint: scrollController);

  @override
  void paint(Canvas canvas, Size size) {
    if (zoomLevel <= 0 || tempo <= 0 || sampleRate <= 0) return;

    // Calculate Intervals
    final double samplesPerBeat = (60.0 / tempo) * sampleRate;
    final double pixelsPerBeat = samplesPerBeat / zoomLevel;

    if (pixelsPerBeat < 1.0) return;

    // Drawing Settings
    final TextPainter textPainter = TextPainter(
      textDirection: TextDirection.ltr,
    );

    final Paint majorTickPaint = Paint()
      ..color = Colors.white54
      ..strokeWidth = 1.0;

    final Paint minorTickPaint = Paint()
      ..color = Colors.white24
      ..strokeWidth = 1.0;

    const int beatsPerBar = 4;
    final double pixelsPerBar = pixelsPerBeat * beatsPerBar;

    // Calculate Visible Range safely
    double startPixel = 0.0;
    double endPixel = size.width;

    // Handle multiple clients safely
    if (scrollController.hasClients) {
      // When a controller is attached to multiple views, .offset throws.
      // We must access specific positions. Since they are synced, taking the first is fine.
      final position = scrollController.positions.first;

      final offset = position.pixels;
      final viewportWidth = position.hasViewportDimension
          ? position.viewportDimension
          : 1000.0;

      const double buffer = 200.0;
      startPixel = (offset - buffer).clamp(0.0, double.infinity);
      endPixel = offset + viewportWidth + buffer;
    }

    // Determine Start Index
    int barIndex = (startPixel / pixelsPerBar).floor();
    if (barIndex < 1) barIndex = 1;

    double currentX = (barIndex - 1) * pixelsPerBar;

    // Draw Loop
    while (currentX < endPixel) {
      if (currentX > size.width) break;

      if (currentX >= startPixel) {
        // Draw Major Tick
        canvas.drawLine(
          Offset(currentX, 15),
          Offset(currentX, size.height),
          majorTickPaint,
        );

        // Draw Bar Number
        textPainter.text = TextSpan(
          text: '$barIndex',
          style: const TextStyle(color: Colors.white70, fontSize: 10),
        );
        textPainter.layout();
        textPainter.paint(canvas, Offset(currentX + 4, 2));
      }

      // Draw Beat Ticks
      if (pixelsPerBeat > 5.0) {
        for (int i = 1; i < beatsPerBar; i++) {
          double beatX = currentX + (pixelsPerBeat * i);

          if (beatX >= startPixel && beatX < endPixel && beatX < size.width) {
            canvas.drawLine(
              Offset(beatX, 22),
              Offset(beatX, size.height),
              minorTickPaint,
            );
          }
        }
      }

      currentX += pixelsPerBar;
      barIndex++;
    }
  }

  @override
  bool shouldRepaint(covariant _TimelineRulerPainter oldDelegate) {
    return oldDelegate.zoomLevel != zoomLevel ||
        oldDelegate.tempo != tempo ||
        oldDelegate.sampleRate != sampleRate ||
        oldDelegate.scrollController != scrollController;
  }
}
