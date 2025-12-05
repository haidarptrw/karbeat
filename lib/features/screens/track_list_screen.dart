import 'package:flutter/material.dart';
import 'package:karbeat/features/playlist/track_slot.dart';
import 'package:karbeat/src/rust/api/project.dart';
import 'package:karbeat/src/rust/core/project.dart';
import 'package:karbeat/state/app_state.dart';
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
  late ScrollController _rulerController;        // Controller 1: Top Ruler
  late ScrollController _trackContentController; // Controller 2: Bottom Conten

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
  }

  @override
  void dispose() {
    _headerController.dispose();
    _timelineController.dispose();
    _rulerController.dispose();
    _trackContentController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    // Calculate total height to ensure both lists have exactly same extent
    // +1 for the Add Button row
    final int itemCount = widget.tracks.length + 1;

    return Row(
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
          child: Column(
            children: [
              // Optional: Time Ruler Header (Horizontal Scrollable)
              // We would need another sync controller for the ruler + body horizontal scroll.
              Container(
                height: 30,
                color: Colors.grey.shade800,
                width: double.infinity,
                child: SingleChildScrollView(
                  scrollDirection: Axis.horizontal,
                  controller: _rulerController,
                  physics: const ClampingScrollPhysics(),
                  child: SizedBox(
                    width: 50000, // Matches track list width
                    height: 30,
                    child: _TimelineRuler(
                      scrollController: _rulerController,
                    ),
                  ),
                ),
              ),
              Expanded(
                child: SingleChildScrollView(
                  scrollDirection: Axis.horizontal, // Horizontal Scroll
                  controller: _trackContentController,
                  // Physics to match desktop feel
                  physics: const ClampingScrollPhysics(),
                  child: SizedBox(
                    width: 50000, // TODO: Bind to project duration
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
                        return KarbeatTrackSlot(
                          trackId: widget.tracks[index].id,
                          height: widget.itemHeight,
                          horizontalScrollController: _trackContentController,
                        );
                      },
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
    showDialog(
      context: context,
      builder: (ctx) => SimpleDialog(
        title: const Text("Select Track Type"),
        children: [
          SimpleDialogOption(
            onPressed: () {
              context.read<KarbeatState>().addTrack(TrackType.audio);
              Navigator.pop(ctx);
            },
            child: const Text("Audio Track"),
          ),
          SimpleDialogOption(
            onPressed: () {
              context.read<KarbeatState>().addTrack(TrackType.midi);
              Navigator.pop(ctx);
            },
            child: const Text("MIDI / Instrument Track"),
          ),
        ],
      ),
    );
  }
}

class _TimelineRuler extends StatelessWidget {
  final ScrollController scrollController;

  const _TimelineRuler({required this.scrollController});

  @override
  Widget build(BuildContext context) {
    // Read state for drawing
    final zoomLevel = context.select<KarbeatState, double>(
      (s) => s.horizontalZoomLevel,
    );
    final tempo = context.select<KarbeatState, double>((s) => s.tempo);
    final sampleRate = context.select<KarbeatState, int>(
      (s) => s.hardwareConfig.sampleRate,
    );
    final safeSampleRate = sampleRate <= 0 ? 44100 : sampleRate;

    return RepaintBoundary(
      child: CustomPaint(
        // FIX 1: Set explicit size to Zero so it fills parent constraints (50,000)
        // instead of trying to be Infinite.
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

    // 1. Calculate Intervals
    final double samplesPerBeat = (60.0 / tempo) * sampleRate;
    final double pixelsPerBeat = samplesPerBeat / zoomLevel;

    if (pixelsPerBeat < 1.0) return;

    // 2. Drawing Settings
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

    // 3. OPTIMIZATION: Calculate Visible Range safely
    double startPixel = 0.0;
    double endPixel = size.width;

    // FIX 2: Handle multiple clients safely
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

    // 4. Determine Start Index
    int barIndex = (startPixel / pixelsPerBar).floor();
    if (barIndex < 1) barIndex = 1;

    double currentX = (barIndex - 1) * pixelsPerBar;

    // 5. Draw Loop
    while (currentX < endPixel) {
      if (currentX > size.width) break;

      if (currentX >= startPixel) {
        // A. Draw Major Tick
        canvas.drawLine(
          Offset(currentX, 15), 
          Offset(currentX, size.height), 
          majorTickPaint
        );

        // B. Draw Bar Number
        textPainter.text = TextSpan(
          text: '$barIndex',
          style: const TextStyle(color: Colors.white70, fontSize: 10),
        );
        textPainter.layout();
        textPainter.paint(canvas, Offset(currentX + 4, 2));
      }

      // C. Draw Beat Ticks
      if (pixelsPerBeat > 5.0) {
        for (int i = 1; i < beatsPerBar; i++) {
          double beatX = currentX + (pixelsPerBeat * i);
          
          if (beatX >= startPixel && beatX < endPixel && beatX < size.width) {
            canvas.drawLine(
              Offset(beatX, 22), 
              Offset(beatX, size.height), 
              minorTickPaint
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