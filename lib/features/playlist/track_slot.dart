import 'dart:ui';

import 'package:flutter/material.dart';
import 'package:flutter/widgets.dart';
import 'package:karbeat/src/rust/api/project.dart';
import 'package:karbeat/src/rust/core/project.dart';
import 'package:karbeat/state/app_state.dart';
import 'package:provider/provider.dart';

class KarbeatTrackSlot extends StatelessWidget {
  final int trackId;
  final double height;

  const KarbeatTrackSlot({
    super.key,
    required this.trackId,
    required this.height,
  });

  @override
  Widget build(BuildContext context) {
    // 1. Listen to Zoom Level (Global)
    // We select specifically the horizontal zoom to rebuild layout when zooming
    final zoomLevel = context.select<KarbeatState, double>(
      (state) => state.horizontalZoomLevel,
    );

    // 2. Listen to Track Data
    // We find our specific track from the map
    final track = context.select<KarbeatState, UiTrack?>(
      (state) => state.tracks[trackId],
    );

    if (track == null) return const SizedBox();

    return Container(
      height: height,
      decoration: BoxDecoration(
        border: Border(
          bottom: BorderSide(color: Colors.white.withAlpha(16), width: 1),
          right: BorderSide(color: Colors.white.withAlpha(16), width: 1),
        ),
        color: Colors.grey.shade900,
      ),
      child: Stack(
        clipBehavior: Clip.none, // Allow clips to drag outside temporarily
        children: track.clips.map((clip) {
          return _buildClipWidget(context, clip, track.trackType, zoomLevel);
        }).toList(),
      ),
    );
  }

  Widget _buildClipWidget(
    BuildContext context,
    UiClip clip,
    TrackType type,
    double samplesPerPixel,
  ) {
    // === COORDINATE MAPPING ===
    // Start (Pixels) = Start (Samples) / Zoom (Samples per Pixel)
    final double left = clip.startTime / samplesPerPixel;
    final double width = clip.loopLength / samplesPerPixel;

    return Positioned(
      left: left,
      top: 2, // Padding top
      height: height - 4, // Padding bottom
      width: width,
      child: _ClipRenderer(
        clip: clip,
        trackType: type,
        // Optional: Pass color based on track or clip settings
        color: Colors.cyanAccent.withAlpha(47),
      ),
    );
  }
}

// =============================================================================
// 2. THE CLIP RENDERER (The actual colored box)
// =============================================================================

class _ClipRenderer extends StatelessWidget {
  final UiClip clip;
  final TrackType trackType;
  final Color color;

  const _ClipRenderer({
    required this.clip,
    required this.trackType,
    required this.color,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      decoration: BoxDecoration(
        color: color,
        borderRadius: BorderRadius.circular(4),
        border: Border.all(color: color.withAlpha(16), width: 1),
      ),
      child: ClipRRect(
        borderRadius: BorderRadius.circular(3),
        child: Stack(
          children: [
            // A. Content (Waveform or MIDI Notes)
            Positioned.fill(child: _buildContent()),

            // B. Label Header
            Positioned(
              top: 0,
              left: 0,
              right: 0,
              height: 16,
              child: Container(
                padding: const EdgeInsets.symmetric(horizontal: 4),
                color: Colors.black26,
                child: Text(
                  clip.name,
                  style: const TextStyle(
                    color: Colors.white,
                    fontSize: 10,
                    fontWeight: FontWeight.w500,
                  ),
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildContent() {
    switch (clip.source) {

      // In future: Use CustomPainter to draw the waveform summary here

      // case KarbeatSource_Midi():
      //   return const Center(
      //     child: Icon(Icons.piano, size: 16, color: Colors.white54),
      //   );
      // // In future: Draw mini rectangles for notes

      // case KarbeatSource_Automation():
      //   return const Center(
      //     child: Icon(Icons.show_chart, size: 16, color: Colors.white54),
      //   );
      case UiClipSource_Audio():
        // TODO In future: Use CustomPainter to draw the waveform summary here
        return const Center(
          child: Icon(Icons.graphic_eq, size: 16, color: Colors.white54),
        );
      case UiClipSource_None():
        return const Center(
          child: Icon(Icons.show_chart, size: 16, color: Colors.white54),
        );
    }
  }
}
