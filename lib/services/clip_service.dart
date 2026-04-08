import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:karbeat/src/rust/api/track.dart' as track_api;
import 'package:karbeat/state/app_state.dart';
import 'package:karbeat/utils/result_type.dart';

class ClipService {
  static Future<Result<Null>> handleCut({
    required int trackId,
    required int clipId,
    required int cutPointSample,
  }) async {
    try {
      await track_api.cutClip(
        sourceTrackId: trackId,
        clipId: clipId,
        cutPointSample: cutPointSample,
      );
      return Result.ok(null);
    } catch (e) {
      return Result.error(Exception(e));
    }
  }
}
