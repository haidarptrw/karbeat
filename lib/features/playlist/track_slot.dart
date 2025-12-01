import 'dart:ui';

import 'package:flutter/widgets.dart';

// Note this state of the rack will be handled by the parent container, any changes made in the backend
// will be informed by it and then get sent to frontend

class TrackSection {
  String name;

  // location by time
  double startTime;
  double endTime;

  // location from beat
  double startBeat;
  double endBeat;

  TrackSection({
    required this.name,
    required this.startTime,
    required this.endTime,
    required this.startBeat,
    required this.endBeat
  });
}

/// Represent AudioFile that will be processed
class AudioFile {
  /// This only store the path to the source file. The actual data of the file will be stored by the backend
  late String sourceFile;
  late double durationMs;
  late int sampleRate;
}

class AudioSampleSection extends TrackSection {
  // ========== CONSTANTS ===============
  static const double _minPan = -100;
  static const double _maxPan = 100;

  AudioFile? audioFile;

  // =============== Common audio controls =================
  double _panValue;
  double _pitchValue;
  /// handles how much range of the pitch value. each unit represent 1 semitone
  int _pitchRange;

  // TODO: Implementing these features
  // ============== Later implemented =============
  // ADSR adsrEnvelope;
  // Arpeggio arpeggio;

  // This dictates the sample method ("stretch" to stretch the audio whenever the
  // audio length changes, preserve the pitch, "normal" will up or downsample the audio, thus
  // increase/decrease the audio pitch)
  // AudioMode mode;

  /// Constructor of Audio Sample Section
  AudioSampleSection({
    required super.name, 
    required super.startTime, 
    required super.endTime, 
    required super.startBeat, 
    required super.endBeat,
    double panValue = 0.0,
    int pitchRange = 12,
    double pitchValue = 0
  }) : _pitchRange = pitchRange, _pitchValue = pitchValue, _panValue = panValue;


  // ========= Getters and Setters =================

  double get panValue {
    return _panValue;
  }

  double get pitchValue {
    return _pitchValue;
  }

  int get pitchRange {
    return _pitchRange;
  }

  set panValue(double value) {
    _panValue = clampDouble(value, _minPan, _maxPan);
    // TODO: Update value at the backend
  }

  set pitchValue(double value) {
    _pitchValue = clampDouble(value, -(_pitchValue), _pitchValue);
    // TODO: Update value at the backend
  }

  set pitchRange(int value) {
    _pitchRange = value.clamp(2, 60);
    // TODO: Update value at the backend
  }
}

class KarbeatTrackSlot extends StatefulWidget{
  const KarbeatTrackSlot({super.key});

  @override
  KarbeatTrackSlotState createState()  => KarbeatTrackSlotState();

}

class KarbeatTrackSlotState extends State<KarbeatTrackSlot> {
  String name;
  double height;

  KarbeatTrackSlotState({
    this.name = "New Track",
    this.height = 180
  });
  // this class will store a map of reference to either audio samples, automation, and MIDI pattern

  // ====== FUTURE FEATURES ======
  // String color

  @override
  Widget build(BuildContext context) {
    // TODO: implement build
    throw UnimplementedError();
  }

}