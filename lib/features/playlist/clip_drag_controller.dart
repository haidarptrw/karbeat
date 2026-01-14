import 'package:flutter/foundation.dart';

/// The type of batch drag action being performed
enum BatchDragAction { none, move, resizeLeft, resizeRight }

/// Controller that coordinates batch drag operations across multiple clips
/// using the leader-follower pattern.
///
/// When a clip that is part of a multi-selection is dragged:
/// 1. That clip becomes the "leader" and calls [startBatchDrag]
/// 2. During drag, the leader calls [updateDelta] to broadcast the delta
/// 3. Other selected clips ("followers") listen to this controller and apply the same delta
/// 4. On drag end, the leader calls [endBatchDrag] which triggers the batch API
class ClipDragController extends ChangeNotifier {
  BatchDragAction _action = BatchDragAction.none;
  int? _leaderClipId;
  int _deltaSamples = 0;
  double _deltaRows = 0.0;

  /// Whether a batch drag is currently active
  bool get isActive => _action != BatchDragAction.none;

  /// The current action being performed
  BatchDragAction get action => _action;

  /// The ID of the clip leading the drag
  int? get leaderClipId => _leaderClipId;

  /// Cumulative horizontal delta in samples
  int get deltaSamples => _deltaSamples;

  /// Cumulative vertical delta in row units (for track switching)
  double get deltaRows => _deltaRows;

  /// Start a batch drag operation with the specified clip as the leader
  void startBatchDrag(int clipId, BatchDragAction action) {
    _leaderClipId = clipId;
    _action = action;
    _deltaSamples = 0;
    _deltaRows = 0.0;
    notifyListeners();
  }

  /// Update the cumulative delta during drag
  void updateDelta(int dSamples, double dRows) {
    _deltaSamples += dSamples;
    _deltaRows += dRows;
    notifyListeners();
  }

  /// End the batch drag operation
  /// The leader should call the appropriate batch API after this
  void endBatchDrag() {
    _action = BatchDragAction.none;
    notifyListeners();
  }

  /// Reset all state (called after API commit)
  void reset() {
    _action = BatchDragAction.none;
    _leaderClipId = null;
    _deltaSamples = 0;
    _deltaRows = 0.0;
    notifyListeners();
  }
}
