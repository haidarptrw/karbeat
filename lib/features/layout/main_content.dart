import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:karbeat/features/header/control_panel.dart';
import 'package:karbeat/features/screens/mixer_screen.dart';
import 'package:karbeat/features/screens/piano_roll_screen.dart';
import 'package:karbeat/features/screens/source_list_screen.dart';
import 'package:karbeat/features/screens/track_list_screen.dart';
import 'package:karbeat/src/rust/api/project.dart';
import 'package:karbeat/state/app_state.dart';
import 'package:karbeat/utils/logger.dart';

class MainContent extends ConsumerWidget {
  const MainContent({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final currentView = ref.watch(
      karbeatStateProvider.select((s) => s.currentView),
    );

    return Container(
      color: Colors.grey.shade800,
      child: Column(
        children: [
          SafeArea(
            top: true,
            bottom: false,
            child: Container(
              color: Colors.grey.shade50,
              child: const DefaultControlPanel(),
            ),
          ),
          Expanded(child: _buildWorkspaceView(context, ref, currentView)),
        ],
      ),
    );
  }

  Widget _buildWorkspaceView(
    BuildContext context,
    WidgetRef ref,
    WorkspaceView currentView,
  ) {
    switch (currentView) {
      case WorkspaceView.trackList:
        return const TrackListScreen();
      case WorkspaceView.source:
        return const SourceListScreen();
      case WorkspaceView.pianoRoll:
        return _buildPianoRoll(ref);
      case WorkspaceView.mixer:
        return const MixerScreen();
    }
  }

  Widget _buildPianoRoll(WidgetRef ref) {
    final state = ref.watch(karbeatStateProvider);

    // Try to get pattern from focused clip (most recently selected)
    final clipId = state.focusClipId;
    final trackId = state.selectedTrackId;
    int? resultPatternId;
    int? generatorId;

    if (clipId != null && trackId != null) {
      final track = state.tracks[trackId];
      if (track != null) {
        for (final clip in track.clips) {
          if (clip.id == clipId) {
            if (clip.source case UiClipSource_Midi(:final patternId)) {
              resultPatternId = patternId;
              generatorId = track.generatorId;
            }
          }
        }
      }
    }

    // Fallback: Use editingPatternId (from source list)
    resultPatternId ??= state.editingPatternId;

    KarbeatLogger.info(
      "Opening piano roll for pattern: $resultPatternId on track: $generatorId",
    );

    return PianoRollScreen(
      patternId: resultPatternId,
      generatorId: generatorId,
    );
  }
}
