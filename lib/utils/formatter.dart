String formatTimeFromSamples(int playheadSamples, int sampleRate) {
  final totalSeconds = playheadSamples / sampleRate;

  final minutes = totalSeconds ~/ 60;
  final seconds = (totalSeconds % 60).floor();

  // Milliseconds (2-digit precision)
  final ms = ((totalSeconds - totalSeconds.floor()) * 100).floor();

  final mm = minutes.toString().padLeft(2, '0');
  final ss = seconds.toString().padLeft(2, '0');
  final mms = ms.toString().padLeft(2, '0');

  return "$mm:$ss:$mms";
}
