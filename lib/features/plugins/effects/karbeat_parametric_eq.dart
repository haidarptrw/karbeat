//                Planned UI Layout
// |                                            |
// |  Response Curve with draggable             |
// |  filter node to edit the param values      |
// |                                            |
// |                                            |
// --------------------------------------------
// | Master | Band 1 | Band 2 | Band 3 | Band 4 |
// |        |        |        |        |        |

import 'package:flutter/material.dart';
import 'dart:math';

import 'package:karbeat/features/plugins/effects/abstract_effect_screen.dart';
import 'package:karbeat/src/rust/api/plugin.dart' as plugin_api;

/// Math helpers for Logarithmic Frequency Mapping
const double minFreq = 20.0;
const double maxFreq = 20000.0;
const double minGain = -24.0;
const double maxGain = 24.0;

double _freqToX(double freq, double width) {
  final minF = log(minFreq) / ln10;
  final maxF = log(maxFreq) / ln10;
  final val = log(freq.clamp(minFreq, maxFreq)) / ln10;
  return ((val - minF) / (maxF - minF)) * width;
}

double _xToFreq(double x, double width) {
  final minF = log(minFreq) / ln10;
  final maxF = log(maxFreq) / ln10;
  final val = (x / width) * (maxF - minF) + minF;
  return pow(10, val).toDouble();
}

double _gainToY(double gain, double height) {
  // Y is inverted (0 at top, height at bottom)
  final normalized =
      (gain.clamp(minGain, maxGain) - minGain) / (maxGain - minGain);
  return height - (normalized * height);
}

double _yToGain(double y, double height) {
  final normalized = 1.0 - (y / height).clamp(0.0, 1.0);
  return (normalized * (maxGain - minGain)) + minGain;
}

/// Data model for an EQ Band matching `parametric_eq.rs`
class EqBand {
  bool active;
  int filterType;
  double freq;
  double gain;
  double q;
  int order; // 0=12dB/oct, 1=24dB, 2=36dB, 3=48dB

  EqBand({
    required this.active,
    required this.filterType,
    required this.freq,
    required this.gain,
    required this.q,
    this.order = 0,
  });
}

class KarbeatParametricEq extends AbstractEffectScreen {
  const KarbeatParametricEq({
    Key? key,
    required super.trackId,
    required super.effectIdx,
  }) : super(key: key);

  @override
  KarbeatParametricEqState createState() => KarbeatParametricEqState();
}

class KarbeatParametricEqState
    extends AbstractEffectScreenState<KarbeatParametricEq> {
  // EQ-specific state
  double masterGain = 0.0;
  late List<EqBand> bands;
  int? _draggingNodeIndex;

  // Backend-computed response curve
  List<plugin_api.UiResponseCurvePoint> _responseCurve = [];

  final List<Color> _bandColors = [
    Colors.redAccent,
    Colors.orangeAccent,
    Colors.yellowAccent,
    Colors.greenAccent,
    Colors.lightBlueAccent,
    Colors.cyanAccent,
    Colors.purpleAccent,
    Colors.pinkAccent,
  ];

  final List<String> _filterTypes = [
    "Peaking",
    "Low Shelf",
    "High Shelf",
    "Low Pass",
    "High Pass",
    "Band Pass",
    "Notch",
  ];

  final List<String> _slopeChoices = [
    "12 dB/oct",
    "24 dB/oct",
    "36 dB/oct",
    "48 dB/oct",
  ];

  @override
  String get effectName => 'Parametric EQ';

  @override
  void initState() {
    super.initState();
    _initDefaultBands();
    _fetchResponseCurve();
  }

  /// Fetch the response curve from the Rust backend
  Future<void> _fetchResponseCurve() async {
    try {
      final curve = await plugin_api.getEqResponseCurve(
        trackId: widget.trackId,
        effectId: widget.effectIdx,
        numPoints: 200,
      );
      if (mounted) {
        setState(() => _responseCurve = curve);
      }
    } catch (e) {
      debugPrint('Error fetching EQ response curve: $e');
    }
  }

  void _initDefaultBands() {
    final defaultFreqs = [
      60.0,
      125.0,
      250.0,
      500.0,
      1000.0,
      2000.0,
      4000.0,
      8000.0,
    ];
    bands = List.generate(8, (i) {
      int type = 0; // Peaking
      if (i == 0) type = 1; // Low Shelf
      if (i == 7) type = 2; // High Shelf

      return EqBand(
        active: true,
        filterType: type,
        freq: defaultFreqs[i],
        gain: 0.0,
        q: 0.707,
      );
    });
  }

  // --- Backend Communication ---

  void _updateMasterGain(double value) {
    setState(() => masterGain = value);
    setParameter(2, value.toDouble());
    _fetchResponseCurve();
  }

  void _updateBandParam(int bandIdx, int paramType, double value) {
    setState(() {
      final band = bands[bandIdx];
      switch (paramType) {
        case 0:
          band.freq = value;
          break;
        case 1:
          band.gain = value;
          break;
        case 2:
          band.q = value;
          break;
        case 3:
          band.active = value > 0.5;
          break;
        case 4:
          band.filterType = value.toInt();
          break;
        case 5:
          band.order = value.toInt();
          break;
      }
    });

    // ID Formula from parametric_eq.rs: base_id = 3 + (band_idx * 6)
    int paramId = 3 + (bandIdx * 6) + paramType;
    setParameter(paramId, value.toDouble());
    _fetchResponseCurve();
  }

  // --- Graph Interaction ---

  void _onGraphPanStart(DragStartDetails details, BoxConstraints constraints) {
    // Find the closest node to the tap
    final localPos = details.localPosition;
    double minDistance = double.infinity;
    int? closestIndex;

    for (int i = 0; i < bands.length; i++) {
      if (!bands[i].active) continue;

      final nx = _freqToX(bands[i].freq, constraints.maxWidth);
      final ny = _gainToY(bands[i].gain, constraints.maxHeight);

      final dist = sqrt(pow(nx - localPos.dx, 2) + pow(ny - localPos.dy, 2));
      if (dist < 30.0 && dist < minDistance) {
        // 30px hit radius
        minDistance = dist;
        closestIndex = i;
      }
    }

    if (closestIndex != null) {
      setState(() => _draggingNodeIndex = closestIndex);
    }
  }

  void _onGraphPanUpdate(
    DragUpdateDetails details,
    BoxConstraints constraints,
  ) {
    if (_draggingNodeIndex == null) return;

    final localPos = details.localPosition;

    // Convert pixels back to values
    final newFreq = _xToFreq(localPos.dx, constraints.maxWidth);
    final newGain = _yToGain(localPos.dy, constraints.maxHeight);

    _updateBandParam(_draggingNodeIndex!, 0, newFreq.clamp(minFreq, maxFreq));
    _updateBandParam(_draggingNodeIndex!, 1, newGain.clamp(minGain, maxGain));
  }

  void _onGraphPanEnd(DragEndDetails details) {
    setState(() => _draggingNodeIndex = null);
  }

  // --- UI Building ---

  @override
  Widget buildEffectBody(BuildContext context) {
    return Container(
      color: Colors.grey.shade900,
      child: Column(
        children: [
          // TOP: Response Curve
          Expanded(
            flex: 3,
            child: Container(
              margin: const EdgeInsets.all(16),
              decoration: BoxDecoration(
                color: const Color(0xFF16213E),
                borderRadius: BorderRadius.circular(12),
                border: Border.all(color: Colors.grey.shade800),
              ),
              child: ClipRRect(
                borderRadius: BorderRadius.circular(12),
                child: LayoutBuilder(
                  builder: (context, constraints) {
                    return GestureDetector(
                      onPanStart: (d) => _onGraphPanStart(d, constraints),
                      onPanUpdate: (d) => _onGraphPanUpdate(d, constraints),
                      onPanEnd: _onGraphPanEnd,
                      child: CustomPaint(
                        size: Size(constraints.maxWidth, constraints.maxHeight),
                        painter: _EqResponsePainter(
                          bands: bands,
                          bandColors: _bandColors,
                          activeNodeIndex: _draggingNodeIndex,
                          responseCurve: _responseCurve,
                        ),
                      ),
                    );
                  },
                ),
              ),
            ),
          ),

          // BOTTOM: Controls
          Expanded(
            flex: 2,
            child: Container(
              padding: const EdgeInsets.symmetric(vertical: 8),
              decoration: BoxDecoration(
                border: Border(top: BorderSide(color: Colors.grey.shade800)),
              ),
              child: Row(
                children: [
                  _buildMasterStrip(),
                  Container(
                    width: 1,
                    color: Colors.grey.shade800,
                    margin: const EdgeInsets.symmetric(horizontal: 8),
                  ),
                  Expanded(
                    child: ListView.builder(
                      scrollDirection: Axis.horizontal,
                      itemCount: bands.length,
                      itemBuilder: (context, index) => _buildBandStrip(index),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildMasterStrip() {
    return Container(
      width: 80,
      padding: const EdgeInsets.all(8),
      child: Column(
        children: [
          const Text(
            "MASTER",
            style: TextStyle(
              color: Colors.white70,
              fontSize: 12,
              fontWeight: FontWeight.bold,
            ),
          ),
          const SizedBox(height: 16),
          Expanded(
            child: RotatedBox(
              quarterTurns: 3,
              child: SliderTheme(
                data: SliderThemeData(
                  trackHeight: 4,
                  thumbShape: const RoundSliderThumbShape(
                    enabledThumbRadius: 8,
                  ),
                ),
                child: Slider(
                  value: masterGain,
                  min: minGain,
                  max: maxGain,
                  activeColor: Colors.white,
                  onChanged: _updateMasterGain,
                ),
              ),
            ),
          ),
          const SizedBox(height: 8),
          Text(
            "${masterGain.toStringAsFixed(1)} dB",
            style: const TextStyle(color: Colors.white54, fontSize: 10),
          ),
        ],
      ),
    );
  }

  Widget _buildBandStrip(int i) {
    final band = bands[i];
    final color = _bandColors[i];

    return Container(
      width: 100,
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 4),
      margin: const EdgeInsets.only(right: 8),
      decoration: BoxDecoration(
        color: Colors.grey.shade800.withAlpha(50),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(
          color: band.active ? color.withAlpha(100) : Colors.transparent,
        ),
      ),
      child: SingleChildScrollView(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.center,
          mainAxisSize: MainAxisSize.min,
          children: [
            // Band Header + Active Toggle
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Container(
                      width: 6,
                      height: 6,
                      decoration: BoxDecoration(
                        shape: BoxShape.circle,
                        color: color,
                      ),
                    ),
                    const SizedBox(width: 3),
                    Text(
                      "B${i + 1}",
                      style: TextStyle(
                        color: band.active ? Colors.white70 : Colors.white30,
                        fontSize: 9,
                        fontWeight: FontWeight.bold,
                      ),
                    ),
                  ],
                ),
                // Compact toggle
                SizedBox(
                  width: 32,
                  height: 20,
                  child: FittedBox(
                    fit: BoxFit.contain,
                    child: Switch(
                      value: band.active,
                      materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
                      activeThumbColor: color,
                      onChanged: (val) =>
                          _updateBandParam(i, 3, val ? 1.0 : 0.0),
                    ),
                  ),
                ),
              ],
            ),

            // Type Dropdown (compact)
            SizedBox(
              height: 28,
              child: DropdownButton<int>(
                value: band.filterType,
                isExpanded: true,
                dropdownColor: Colors.grey.shade800,
                style: const TextStyle(color: Colors.white54, fontSize: 9),
                underline: const SizedBox(),
                isDense: true,
                onChanged: (val) => _updateBandParam(i, 4, val!.toDouble()),
                items: List.generate(_filterTypes.length, (idx) {
                  return DropdownMenuItem(
                    value: idx,
                    child: Text(_filterTypes[idx]),
                  );
                }),
              ),
            ),

            const SizedBox(height: 2),

            // Parameter controls
            _buildParamControl(
              "Freq",
              band.freq,
              minFreq,
              maxFreq,
              (v) => _updateBandParam(i, 0, v),
              isLog: true,
              suffix: "Hz",
            ),
            _buildParamControl(
              "Gain",
              band.gain,
              minGain,
              maxGain,
              (v) => _updateBandParam(i, 1, v),
              suffix: "dB",
            ),
            _buildParamControl(
              "Q",
              band.q,
              0.1,
              20.0,
              (v) => _updateBandParam(i, 2, v),
              suffix: "",
            ),

            // Slope dropdown
            const SizedBox(height: 4),
            SizedBox(
              height: 28,
              child: DropdownButton<int>(
                value: band.order.clamp(0, _slopeChoices.length - 1),
                isExpanded: true,
                dropdownColor: Colors.grey.shade800,
                style: const TextStyle(color: Colors.white54, fontSize: 9),
                underline: const SizedBox(),
                isDense: true,
                onChanged: (val) => _updateBandParam(i, 5, val!.toDouble()),
                items: List.generate(_slopeChoices.length, (idx) {
                  return DropdownMenuItem(
                    value: idx,
                    child: Text(_slopeChoices[idx]),
                  );
                }),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildParamControl(
    String label,
    double val,
    double min,
    double max,
    Function(double) onChanged, {
    bool isLog = false,
    String suffix = "",
  }) {
    return Column(
      children: [
        Text(label, style: const TextStyle(color: Colors.grey, fontSize: 9)),
        SliderTheme(
          data: SliderThemeData(
            trackHeight: 2,
            thumbShape: const RoundSliderThumbShape(enabledThumbRadius: 5),
            overlayShape: const RoundSliderOverlayShape(overlayRadius: 10),
          ),
          child: Slider(
            value: (isLog ? log(val) / ln10 : val).clamp(
              isLog ? log(min) / ln10 : min,
              isLog ? log(max) / ln10 : max,
            ),
            min: isLog ? log(min) / ln10 : min,
            max: isLog ? log(max) / ln10 : max,
            onChanged: (newVal) =>
                onChanged(isLog ? pow(10, newVal).toDouble() : newVal),
          ),
        ),
        Text(
          "${val >= 1000 ? '${(val / 1000).toStringAsFixed(1)}k' : val.toStringAsFixed(1)}$suffix",
          style: const TextStyle(color: Colors.white, fontSize: 9),
        ),
      ],
    );
  }
}

/// Custom graph painter to render the frequency response curve and interactable nodes
class _EqResponsePainter extends CustomPainter {
  final List<EqBand> bands;
  final List<Color> bandColors;
  final int? activeNodeIndex;
  final List<plugin_api.UiResponseCurvePoint> responseCurve;

  _EqResponsePainter({
    required this.bands,
    required this.bandColors,
    required this.activeNodeIndex,
    required this.responseCurve,
  });

  @override
  void paint(Canvas canvas, Size size) {
    final w = size.width;
    final h = size.height;

    // 1. Draw Grid Lines
    final gridPaint = Paint()
      ..color = Colors.white.withAlpha(20)
      ..strokeWidth = 1;
    final textPainter = TextPainter(textDirection: TextDirection.ltr);

    final freqsToDraw = [50.0, 100.0, 500.0, 1000.0, 5000.0, 10000.0];
    for (var f in freqsToDraw) {
      final x = _freqToX(f, w);
      canvas.drawLine(Offset(x, 0), Offset(x, h), gridPaint);

      textPainter.text = TextSpan(
        text: f >= 1000 ? "${f ~/ 1000}k" : "${f.toInt()}",
        style: const TextStyle(color: Colors.white30, fontSize: 10),
      );
      textPainter.layout();
      textPainter.paint(canvas, Offset(x + 2, h - 14));
    }

    // 0dB Center Line
    canvas.drawLine(
      Offset(0, h / 2),
      Offset(w, h / 2),
      Paint()
        ..color = Colors.white54
        ..strokeWidth = 1,
    );

    // 2. Draw the response curve from backend-computed data
    if (responseCurve.isNotEmpty) {
      final path = Path();

      for (int i = 0; i < responseCurve.length; i++) {
        final point = responseCurve[i];
        final x = _freqToX(point.frequency.toDouble(), w);
        final y = _gainToY(
          point.magnitudeDb.toDouble().clamp(minGain, maxGain),
          h,
        );

        if (i == 0) {
          path.moveTo(x, y);
        } else {
          path.lineTo(x, y);
        }
      }

      // Fill under curve
      final fillPath = Path.from(path)
        ..lineTo(w, h / 2)
        ..lineTo(0, h / 2)
        ..close();

      canvas.drawPath(
        fillPath,
        Paint()
          ..color = Colors.cyanAccent.withAlpha(20)
          ..style = PaintingStyle.fill,
      );

      // Stroke curve
      canvas.drawPath(
        path,
        Paint()
          ..color = Colors.cyanAccent
          ..strokeWidth = 2
          ..style = PaintingStyle.stroke,
      );
    }

    // 3. Draw Interactable Nodes
    for (int i = 0; i < bands.length; i++) {
      if (!bands[i].active) continue;

      final x = _freqToX(bands[i].freq, w);
      final y = _gainToY(bands[i].gain, h);
      final color = bandColors[i];
      final isDragging = activeNodeIndex == i;

      // Draw vertical drop line
      canvas.drawLine(
        Offset(x, y),
        Offset(x, h / 2),
        Paint()
          ..color = color.withAlpha(isDragging ? 150 : 50)
          ..strokeWidth = 1,
      );

      // Node Circle
      canvas.drawCircle(
        Offset(x, y),
        isDragging ? 8 : 6,
        Paint()
          ..color = color
          ..style = PaintingStyle.fill,
      );
      canvas.drawCircle(
        Offset(x, y),
        isDragging ? 8 : 6,
        Paint()
          ..color = Colors.white
          ..style = PaintingStyle.stroke
          ..strokeWidth = 1,
      );

      // Draw Band Number
      textPainter.text = TextSpan(
        text: "${i + 1}",
        style: const TextStyle(
          color: Colors.black,
          fontSize: 9,
          fontWeight: FontWeight.bold,
        ),
      );
      textPainter.layout();
      textPainter.paint(
        canvas,
        Offset(x - textPainter.width / 2, y - textPainter.height / 2),
      );
    }
  }

  @override
  bool shouldRepaint(covariant _EqResponsePainter oldDelegate) => true; // Ideally check for actual changes
}
