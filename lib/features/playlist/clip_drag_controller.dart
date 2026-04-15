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
  BatchDragAction action = BatchDragAction.none;
  int snappedDeltaSamples = 0;
  double verticalDragDy = 0.0;

  bool get isActive => action != BatchDragAction.none;

  void startBatchDrag(BatchDragAction newAction) {
    action = newAction;
    snappedDeltaSamples = 0;
    verticalDragDy = 0.0;
    notifyListeners();
  }

  void updateDrag(int deltaSamples, double dy) {
    snappedDeltaSamples = deltaSamples;
    verticalDragDy = dy;
    notifyListeners();
  }

  void reset() {
    action = BatchDragAction.none;
    snappedDeltaSamples = 0;
    verticalDragDy = 0.0;
    notifyListeners();
  }
}
