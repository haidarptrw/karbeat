import 'package:flutter/material.dart';
import 'package:karbeat/src/rust/api/pattern.dart';
import 'package:karbeat/state/app_state.dart';
import 'package:linked_scroll_controller/linked_scroll_controller.dart';
import 'package:provider/provider.dart';

class PianoRollScreen extends StatefulWidget {
  final int? patternId;

  const PianoRollScreen({super.key, this.patternId});

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
        Container(
          height: 40,
          color: Colors.grey.shade800,
          padding: const EdgeInsets.symmetric(horizontal: 10),
          child: Row(
            children: [
              Text(
                pattern.name,
                style: const TextStyle(
                  color: Colors.white,
                  fontWeight: FontWeight.bold,
                ),
              ),
              const Spacer(),
              IconButton(
                icon: const Icon(Icons.zoom_in),
                onPressed: () => setState(() => _zoomX *= 1.2),
              ),
              IconButton(
                icon: const Icon(Icons.zoom_out),
                onPressed: () => setState(() => _zoomX /= 1.2),
              ),
            ],
          ),
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
                    final isBlack = _isBlackKey(midiKey);
                    final label = _getNoteName(midiKey);
                    return Container(
                      height: _keyHeight,
                      decoration: BoxDecoration(
                        color: isBlack ? Colors.black : Colors.white,
                        border: Border(
                          bottom: BorderSide(
                            color: Colors.grey.shade700,
                            width: 0.5,
                          ),
                        ),
                      ),
                      alignment: Alignment.centerRight,
                      padding: const EdgeInsets.only(right: 4),
                      child: (midiKey % 12 == 0 || !isBlack) 
                          ? Text(label, style: TextStyle(fontSize: 9, color: isBlack ? Colors.white54 : Colors.black54))
                          : null,
                    );
                  },
                ),
              ),

              // GRID & NOTES
              Expanded(
                child: SingleChildScrollView(
                  controller: _gridHorizontalController,
                  scrollDirection: Axis.horizontal,
                  child: SingleChildScrollView(
                    controller: _gridVerticalController,
                    scrollDirection: Axis.vertical,
                    child: SizedBox(
                      height: 128 * _keyHeight,
                      width:
                          (pattern.lengthTicks) * _zoomX + 1000, // Approx width
                      child: Stack(
                        children: [
                          // Grid background
                          Positioned.fill(
                            child: CustomPaint(
                              painter: _PianoGridPainter(
                                zoomX: _zoomX,
                                keyHeight: _keyHeight,
                              ),
                            ),
                          ),
                          // Notes
                          ...pattern.notes.map(
                            (note) => _buildNoteWidget(note),
                          ),
                        ],
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

  bool _isBlackKey(int key) {
    const blackKey = [1, 3, 6, 8, 10];
    return blackKey.contains(key % 12);
  }

  // Helper for correct Note Names
  String _getNoteName(int midiKey) {
    const names = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'];
    final octave = (midiKey / 12).floor() - 1;
    final name = names[midiKey % 12];
    return "$name$octave";
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
