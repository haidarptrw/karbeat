import 'package:flutter/material.dart';
import 'package:karbeat/src/rust/api/pattern.dart';
import 'package:karbeat/state/app_state.dart';
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
  double _zoomX = 1.0;

  late ScrollController _verticalController;
  late ScrollController _horizontalController;

  @override
  void initState() {
    super.initState();
    _verticalController = ScrollController(
      initialScrollOffset: _keyHeight * 40,
    ); // start middle C
    _horizontalController = ScrollController();
  }

  @override
  void dispose() {
    _verticalController.dispose();
    _horizontalController.dispose();
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
                  controller: _verticalController,
                  itemCount: 128,
                  itemBuilder: (context, index) {
                    // MIDI 127 is top, 0 is bottom. List index 0 is top.
                    final midiKey = 127 - index;
                    final isBlack = _isBlackKey(midiKey);
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
                      child: isBlack
                          ? null
                          : Text(
                              "C${(midiKey / 12).floor() - 1}",
                              style: const TextStyle(
                                fontSize: 8,
                                color: Colors.black,
                              ),
                            ),
                    );
                  },
                ),
              ),

              // GRID & NOTES
              Expanded(
                child: SingleChildScrollView(
                  controller: _horizontalController,
                  scrollDirection: Axis.horizontal,
                  child: SingleChildScrollView(
                    controller: _verticalController,
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
                          ...pattern.notes.map((note) => _buildNoteWidget(note)),
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
    final pixelsPerBeat = 960 * zoomX;
    int beats = (size.width / pixelsPerBeat).ceil();

    for (int i = 0; i < beats; i++) {
      final x = i * pixelsPerBeat;
      paint.color = (i % 4 == 0)
          ? Colors.white24
          : Colors.white10; // Bar vs Beat
      canvas.drawLine(Offset(x, 0), Offset(x, size.height), paint);
    }
  }

  @override
  bool shouldRepaint(covariant _PianoGridPainter old) => old.zoomX != zoomX;
}
