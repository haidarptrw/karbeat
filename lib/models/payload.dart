// =================================
// PAYLOAD TYPES
// =================================

class NewNotePayload {
  final int patternId;
  final int key;
  final int startTick;
  final int duration;

  const NewNotePayload({
    required this.patternId,
    required this.key,
    required this.startTick,
    required this.duration,
  });
}

/// Local representation of an EQ point
class CurvePoint {
  final double frequency;
  final double magnitudeDb;

  CurvePoint({required this.frequency, required this.magnitudeDb});
}
