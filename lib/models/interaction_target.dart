/// Model classes representing what the user is interacting with via the selection tool.
/// Used to show the interaction panel with contextual actions.

import 'package:flutter/widgets.dart';

/// Base class for all interaction targets
sealed class InteractionTarget {
  /// Position where the user tapped (for popover positioning on desktop)
  final Offset tapPosition;

  const InteractionTarget({required this.tapPosition});
}

/// Single clip interaction
class ClipInteraction extends InteractionTarget {
  final int trackId;
  final int clipId;

  const ClipInteraction({
    required this.trackId,
    required this.clipId,
    required super.tapPosition,
  });
}

/// Multiple clips selected interaction
class MultiClipInteraction extends InteractionTarget {
  final int trackId;
  final List<int> clipIds;

  const MultiClipInteraction({
    required this.trackId,
    required this.clipIds,
    required super.tapPosition,
  });
}

/// Track header interaction
class TrackInteraction extends InteractionTarget {
  final int trackId;

  const TrackInteraction({required this.trackId, required super.tapPosition});
}
