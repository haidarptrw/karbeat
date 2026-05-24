import 'dart:async';
import 'dart:ffi' as ffi;

import 'package:flutter/material.dart';
import 'dart:math';

import 'package:karbeat/features/audio_plugins/effects/abstract_effect_screen.dart';
import 'package:karbeat/features/components/plugin_parameter_widget.dart';
import 'package:karbeat/models/payload.dart';
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
  final normalized = (gain - minGain) / (maxGain - minGain);
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
    super.key,
    required super.target,
    required super.effectId,
  });

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
  List<CurvePoint> _responseCurve = [];
  List<CurvePoint> _spectrumCurve = [];

  // ======================================
  // Real-time Plugin Command Stream
  // ======================================

  /// Subscription to the plugin command response stream.
  StreamSubscription<plugin_api.UiZeroCopyBufferResponse>? _zeroCopyStreamSub;

  /// request_id of the most recently dispatched GET_MAGNITUDE_RESPONSE command.
  /// Responses are matched by ID so stale replies are ignored.
  int? _magnitudeRequestId;

  /// request_id of the most recently dispatched GET_SPECTRUM command.
  int? _spectrumRequestId;

  /// Timer that re-fires GET_SPECTRUM at a FPS to drive the analyzer.
  Timer? _spectrumPollTimer;

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

  @override
  String get effectName => 'Parametric EQ';

  @override
  void initState() {
    super.initState();
    _initBandsFromParameters();
    _zeroCopyStreamSub = plugin_api.createZeroCopyBufferStream().listen(
      _onZeroCopyMessage,
    );

    // Request the initial magnitude response after the first frame so that
    // the widget.target and widget.effectId are fully bound.
    WidgetsBinding.instance.addPostFrameCallback(
      (_) => _sendMagnitudeRequest(),
    );

    // Re-fire GET_SPECTRUM at ~30 FPS to drive the real-time spectrum analyzer.
    _spectrumPollTimer = Timer.periodic(
      const Duration(milliseconds: 33),
      (_) => _sendSpectrumRequest(),
    );
  }

  @override
  void dispose() {
    _zeroCopyStreamSub?.cancel();
    _spectrumPollTimer?.cancel();
    super.dispose();
  }

  // ======================================
  // Real-time Plugin Command Helpers
  // ======================================

  /// Converts the abstract [UiEffectTarget] + effectId pair into a
  /// [UiPluginTarget] understood by the real-time command channel.
  plugin_api.UiPluginTarget _toPluginTarget() {
    final t = widget.target;
    final eid = widget.effectId;
    if (t is plugin_api.UiEffectTarget_Track) {
      return plugin_api.UiPluginTarget.trackEffect(
        trackId: t.field0,
        effectId: eid,
      );
    } else if (t is plugin_api.UiEffectTarget_Bus) {
      return plugin_api.UiPluginTarget.busEffect(
        busId: t.field0,
        effectId: eid,
      );
    } else {
      return plugin_api.UiPluginTarget.masterEffect(eid);
    }
  }

  /// Sends GET_MAGNITUDE_RESPONSE to the live plugin instance.
  /// The response arrives via [_onPluginMessage].
  void _sendMagnitudeRequest() {
    plugin_api
        .queryLivePluginZeroCopyBuf(
          target: _toPluginTarget(),
          name: 'magnitude',
        )
        .then((id) {
          _magnitudeRequestId = id;
        })
        .catchError((Object e) {
          debugPrint('EQ magnitude request failed: $e');
        });
  }

  /// Sends GET_SPECTRUM to the live plugin instance.
  /// Fired at ~30 FPS by [_spectrumPollTimer].
  void _sendSpectrumRequest() {
    plugin_api
        .queryLivePluginZeroCopyBuf(target: _toPluginTarget(), name: 'spectrum')
        .then((id) {
          _spectrumRequestId = id;
        })
        .catchError((Object _) {
          /* Ignore dropped frames */
        });
  }

  /// Routes incoming zero-copy buffer responses to the correct state field.
  /// Converts the raw memory pointer instantly to a Float32List.
  void _onZeroCopyMessage(plugin_api.UiZeroCopyBufferResponse msg) {
    if (!mounted) return;

    if (msg.requestId == _magnitudeRequestId ||
        msg.requestId == _spectrumRequestId) {
      final handle = msg.handle;
      if (handle == null) return;

      // Bridge the raw memory pointer directly to a Dart TypedData list
      final address = handle.memoryAddress();
      final length = handle.lengthElements();

      final ptr = ffi.Pointer<ffi.Float>.fromAddress(address);
      final rawList = ptr.asTypedList(length);

      final List<CurvePoint> parsedPoints = [];

      // Iterate by 2 to extract the pairs [freq, db]
      for (int i = 0; i < rawList.length; i += 2) {
        if (i + 1 < rawList.length) {
          parsedPoints.add(
            CurvePoint(
              frequency: rawList[i].toDouble(),
              magnitudeDb: rawList[i + 1].toDouble(),
            ),
          );
        }
      }

      if (msg.requestId == _magnitudeRequestId) {
        setState(() => _responseCurve = parsedPoints);
      } else {
        setState(() => _spectrumCurve = parsedPoints);
      }
    }
  }

  @override
  void onParametersUpdated() {
    if (parameters.isEmpty) return;

    setState(() {
      _applyParametersToState();
    });

    _sendMagnitudeRequest();
  }

  void _initBandsFromParameters() {
    // Determine how many bands the API actually provided by scanning the paths
    int maxBandIndex = -1;
    for (final p in parameters) {
      final match = RegExp(r'band(\d+)/').firstMatch(p.path);
      if (match != null) {
        final idx = int.parse(match.group(1)!);
        if (idx > maxBandIndex) maxBandIndex = idx;
      }
    }

    // Create the exact number of bands (fallback to 8 if none found)
    final numBands = maxBandIndex >= 0 ? maxBandIndex + 1 : 8;

    // Initialize with generic safe defaults
    bands = List.generate(
      numBands,
      (i) => EqBand(
        active: true,
        filterType: 0,
        freq: 1000.0,
        gain: 0.0,
        q: 0.707,
      ),
    );

    // Immediately apply the actual API values from the parameters list
    _applyParametersToState();
  }

  void _applyParametersToState() {
    for (final p in parameters) {
      // 1. Direct path check for base_gain
      if (p.path == 'base_gain') {
        masterGain = p.value;
        continue;
      }

      // 2. Extract the band index and parameter name from the string path
      final match = RegExp(r'band(\d+)/(.+)').firstMatch(p.path);
      if (match != null) {
        final bandIndex = int.parse(match.group(1)!);

        if (bandIndex >= 0 && bandIndex < bands.length) {
          final band = bands[bandIndex];
          final paramName = match.group(2)!;

          switch (paramName) {
            case 'active':
              band.active = p.value > 0.5;
              break;
            case 'type':
              band.filterType = p.value.toInt();
              break;
            case 'freq':
              band.freq = p.value;
              break;
            case 'q':
              band.q = p.value;
              break;
            case 'gain':
              band.gain = p.value;
              break;
            case 'slope':
              band.order = p.value.toInt();
              break;
          }
        }
      }
    }
  }

  // ==== Backend Communication ====

  void _updateMasterGain(double value) {
    setState(() => masterGain = value);
    setParameterString('base_gain', value);
    _sendMagnitudeRequest();
  }

  void _updateBandParam(int bandIdx, int paramType, double value) {
    String suffix = "";

    setState(() {
      final band = bands[bandIdx];
      switch (paramType) {
        case 0:
          band.freq = value;
          suffix = "freq";
          break;
        case 1:
          band.gain = value;
          suffix = "gain";
          break;
        case 2:
          band.q = value;
          suffix = "q";
          break;
        case 3:
          band.active = value > 0.5;
          suffix = "active";
          break;
        case 4:
          band.filterType = value.toInt();
          suffix = "type";
          break;
        case 5:
          band.order = value.toInt();
          suffix = "slope";
          break;
      }
    });

    // Easily reconstruct the string path mapped in the JSON manifest
    final path = "band$bandIdx/$suffix";
    setParameterString(path, value);
    _sendMagnitudeRequest();
  }

  // === Graph Interaction ===

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
                          spectrumCurve: _spectrumCurve,
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
    plugin_api.UiPluginParameter? p;
    for (final param in parameters) {
      if (param.path == 'base_gain') {
        p = param;
        break;
      }
    }

    final pMin = p?.min ?? minGain;
    final pMax = p?.max ?? maxGain;
    final pDef = p?.defaultValue ?? 0.0;
    final pStep = p?.step ?? 0.1;
    final pName = p?.name ?? 'Gain';
    // the fallback ID is not an invalid ID because at the Rust side, the ID is represented by u32.
    // this will probably crash app if we did not handle it gracefully
    final pId = p?.id ?? -1;
    final pValue = p?.value ?? 0.0;

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
              child: DawFloatParam(
                paramId: pId,
                name: pName,
                value: pValue,
                min: pMin,
                max: pMax,
                defaultValue: pDef,
                step: pStep,
                suffix: "dB",
                onChanged: _updateMasterGain,
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
    // Guard: wrap index so extra bands cycle through colors safely
    final color = _bandColors[i % _bandColors.length];

    // Extract enum choices natively from the Rust parameters
    plugin_api.UiPluginParameter? typeParam;
    plugin_api.UiPluginParameter? slopeParam;
    for (final p in parameters) {
      if (p.path == 'band$i/type') typeParam = p;
      if (p.path == 'band$i/slope') slopeParam = p;
    }

    final filterChoices = typeParam?.choices ?? [];
    final slopeChoices = slopeParam?.choices ?? [];

    return Container(
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
        child: IntrinsicWidth(
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
                child: PopupMenuButton<int>(
                  initialValue: band.filterType,
                  padding: EdgeInsets.zero,
                  color: Colors.grey.shade800,
                  onSelected: (val) => _updateBandParam(i, 4, val.toDouble()),
                  itemBuilder: (context) => List.generate(
                    filterChoices.length,
                    (idx) => PopupMenuItem<int>(
                      value: idx,
                      height: 32,
                      child: Text(
                        filterChoices[idx],
                        style: const TextStyle(
                          color: Colors.white70,
                          fontSize: 9,
                        ),
                      ),
                    ),
                  ),
                  child: Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      Expanded(
                        child: Text(
                          filterChoices.isNotEmpty &&
                                  band.filterType < filterChoices.length
                              ? filterChoices[band.filterType]
                              : "",
                          style: const TextStyle(
                            color: Colors.white54,
                            fontSize: 9,
                          ),
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                      const Icon(
                        Icons.arrow_drop_down,
                        size: 12,
                        color: Colors.white54,
                      ),
                    ],
                  ),
                ),
              ),
          
              const SizedBox(height: 2),
          
              // Parameter controls
              _buildParamControl(
                "Freq",
                band.freq,
                "band$i/freq", // Matches backend dynamic path
                (v) => _updateBandParam(i, 0, v),
                suffix: "Hz",
                parameterName: "Frequency",
              ),
              _buildParamControl(
                "Gain",
                band.gain,
                "band$i/gain", // Matches backend dynamic path
                (v) => _updateBandParam(i, 1, v),
                suffix: "dB",
                parameterName: "Gain",
              ),
              _buildParamControl(
                "Q",
                band.q,
                "band$i/q", // Matches backend dynamic path
                (v) => _updateBandParam(i, 2, v),
                suffix: "",
                parameterName: "Q Bandwidth",
              ),
          
              // Slope dropdown
              const SizedBox(height: 4),
              SizedBox(
                height: 28,
                child: PopupMenuButton<int>(
                  initialValue: band.order,
                  padding: EdgeInsets.zero,
                  color: Colors.grey.shade800,
                  onSelected: (val) => _updateBandParam(i, 5, val.toDouble()),
                  itemBuilder: (context) => List.generate(
                    slopeChoices.length,
                    (idx) => PopupMenuItem<int>(
                      value: idx,
                      height: 32,
                      child: Text(
                        slopeChoices[idx],
                        style: const TextStyle(
                          color: Colors.white70,
                          fontSize: 9,
                        ),
                      ),
                    ),
                  ),
                  child: Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      Expanded(
                        child: Text(
                          slopeChoices.isNotEmpty &&
                                  band.order < slopeChoices.length
                              ? slopeChoices[band.order]
                              : "",
                          style: const TextStyle(
                            color: Colors.white54,
                            fontSize: 9,
                          ),
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                      const Icon(
                        Icons.arrow_drop_down,
                        size: 12,
                        color: Colors.white54,
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildParamControl(
    String label,
    double val,
    String paramPath,
    ValueChanged<double> onChanged, {
    String suffix = "",
    String parameterName = "",
  }) {
    plugin_api.UiPluginParameter? p;
    for (final param in parameters) {
      if (param.path == paramPath) {
        p = param;
        break;
      }
    }

    final pMin = p?.min ?? 0.0;
    final pMax = p?.max ?? 1.0;
    final pDef = p?.defaultValue ?? 0.0;
    final pStep = p?.step ?? 0.1;
    final pId = p?.id ?? -1;

    return Padding(
      padding: const EdgeInsets.only(bottom: 6.0),
      child: DawFloatParam(
        paramId: pId,
        name: label,
        value: val,
        min: pMin,
        max: pMax,
        defaultValue: pDef,
        step: pStep,
        suffix: suffix,
        onChanged: onChanged,
      ),
    );
  }
}

/// Custom graph painter to render the frequency response curve and interactable nodes
class _EqResponsePainter extends CustomPainter {
  final List<EqBand> bands;
  final List<Color> bandColors;
  final int? activeNodeIndex;
  final List<CurvePoint> responseCurve;
  final List<CurvePoint> spectrumCurve;

  _EqResponsePainter({
    required this.bands,
    required this.bandColors,
    required this.activeNodeIndex,
    required this.responseCurve,
    required this.spectrumCurve,
  });

  @override
  void paint(Canvas canvas, Size size) {
    final w = size.width;
    final h = size.height;

    // Draw Grid Lines
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

    // Draw FFT Spectrum Analyzer
    if (spectrumCurve.isNotEmpty) {
      final pts = spectrumCurve.map((p) {
        final x = _freqToX(p.frequency.toDouble(), w);
        final normalizedDb =
            (p.magnitudeDb.toDouble() - (-100.0)) / (24.0 - (-100.0));
        final y = h - (normalizedDb.clamp(0.0, 1.0) * h);
        return Offset(x, y);
      }).toList();

      final path = _buildSmoothPath(pts);

      final fillPath = Path.from(path)
        ..lineTo(w, h)
        ..lineTo(0, h)
        ..close();

      canvas.drawPath(
        fillPath,
        Paint()
          ..shader = LinearGradient(
            begin: Alignment.topCenter,
            end: Alignment.bottomCenter,
            colors: [Colors.white.withAlpha(120), Colors.white.withAlpha(20)],
          ).createShader(Rect.fromLTWH(0, 0, w, h))
          ..style = PaintingStyle.fill,
      );

      canvas.drawPath(
        path,
        Paint()
          ..color = Colors.white.withAlpha(150)
          ..strokeWidth = 1.0
          ..style = PaintingStyle.stroke
          ..isAntiAlias = true,
      );
    }

    // Draw the response curve from backend-computed data
    if (responseCurve.isNotEmpty) {
      final pts = responseCurve.map((point) {
        final x = _freqToX(point.frequency.toDouble(), w);
        final y = _gainToY(
          point.magnitudeDb.toDouble().clamp(minGain, maxGain),
          h,
        );
        return Offset(x, y);
      }).toList();

      final path = _buildSmoothPath(pts);

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

      canvas.drawPath(
        path,
        Paint()
          ..color = Colors.cyanAccent
          ..strokeWidth = 2
          ..style = PaintingStyle.stroke
          ..isAntiAlias = true,
      );
    }

    // Draw Interactable Nodes
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
  bool shouldRepaint(covariant _EqResponsePainter oldDelegate) => true;
}

Path _buildSmoothPath(List<Offset> pts) {
  final path = Path();
  if (pts.isEmpty) return path;
  if (pts.length == 1) {
    path.addOval(Rect.fromCircle(center: pts[0], radius: 1));
    return path;
  }

  path.moveTo(pts[0].dx, pts[0].dy);

  Offset pAt(int i) {
    if (i < 0) return pts.first;
    if (i >= pts.length) return pts.last;
    return pts[i];
  }

  for (int i = 0; i < pts.length - 1; i++) {
    final p0 = pAt(i - 1);
    final p1 = pAt(i);
    final p2 = pAt(i + 1);
    final p3 = pAt(i + 2);

    // Catmull-Rom -> cubic Bezier
    final c1 = Offset(
      p1.dx + (p2.dx - p0.dx) / 6.0,
      p1.dy + (p2.dy - p0.dy) / 6.0,
    );
    final c2 = Offset(
      p2.dx - (p3.dx - p1.dx) / 6.0,
      p2.dy - (p3.dy - p1.dy) / 6.0,
    );

    path.cubicTo(c1.dx, c1.dy, c2.dx, c2.dy, p2.dx, p2.dy);
  }

  return path;
}
