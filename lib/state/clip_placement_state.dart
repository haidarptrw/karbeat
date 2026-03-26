import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:karbeat/src/rust/api/track.dart';
import 'package:karbeat/state/app_state.dart';
import 'package:karbeat/utils/logger.dart';
import 'package:karbeat/utils/result_type.dart';

class ClipPlacementState {
  final int? sourceId;
  final UiSourceType? sourceType;
  final int trackId;
  final double timeSamples;

  const ClipPlacementState({
    this.sourceId,
    this.sourceType,
    this.trackId = -1,
    this.timeSamples = 0.0,
  });

  bool get isPlacing => sourceId != null;

  ClipPlacementState copyWith({
    int? sourceId,
    UiSourceType? sourceType,
    int? trackId,
    double? timeSamples,
  }) {
    return ClipPlacementState(
      sourceId: sourceId ?? this.sourceId,
      sourceType: sourceType ?? this.sourceType,
      trackId: trackId ?? this.trackId,
      timeSamples: timeSamples ?? this.timeSamples,
    );
  }
}

class ClipPlacementNotifier extends Notifier<ClipPlacementState> {
  @override
  ClipPlacementState build() => const ClipPlacementState();

  void startPlacement(int sourceId, {required UiSourceType type}) {
    state = ClipPlacementState(sourceId: sourceId, sourceType: type);
    ref.read(karbeatStateProvider).navigateTo(WorkspaceView.trackList);
  }

  void updatePlacementTarget(int trackId, double timeSamples) {
    state = state.copyWith(trackId: trackId, timeSamples: timeSamples);
  }

  void cancelPlacement() {
    state = const ClipPlacementState();
  }

  Future<Result<void>> confirmPlacement() async {
    final s = state;
    KarbeatLogger.info("CONFIRM Placement");
    if (s.sourceId != null && s.sourceType != null && s.trackId != -1) {
      try {
        await createClip(
          sourceId: s.sourceId!,
          sourceType: s.sourceType!,
          trackId: s.trackId,
          startTime: s.timeSamples.toInt(),
        );
        ref.read(karbeatStateProvider).notifyBackendChange(ProjectEvent.tracksChanged);
        state = const ClipPlacementState();
        return Result.ok(null);
      } catch (e) {
        KarbeatLogger.error("Error creating clip: $e");
        return Result.error(Exception("$e"));
      }
    }
    return Result.ok(null);
  }
}

final clipPlacementProvider = NotifierProvider<ClipPlacementNotifier, ClipPlacementState>(
  () => ClipPlacementNotifier(),
);
