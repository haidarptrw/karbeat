import 'package:flutter/material.dart';
import 'package:karbeat/models/interaction_target.dart';
import 'package:karbeat/src/rust/api/project.dart';
import 'package:karbeat/src/rust/core/project/track.dart';
import 'package:karbeat/state/app_state.dart';
import 'package:provider/provider.dart';

/// Panel that shows contextual information and actions for selected items.
/// Appears when using the selection tool to tap on clips or tracks.
class InteractionPanel extends StatelessWidget {
  final InteractionTarget target;
  final VoidCallback onClose;

  const InteractionPanel({
    super.key,
    required this.target,
    required this.onClose,
  });

  @override
  Widget build(BuildContext context) {
    return switch (target) {
      ClipInteraction(:final trackId, :final clipId) => _ClipPanel(
        trackId: trackId,
        clipId: clipId,
        onClose: onClose,
      ),
      MultiClipInteraction(:final trackId, :final clipIds) => _MultiClipPanel(
        trackId: trackId,
        clipIds: clipIds,
        onClose: onClose,
      ),
      TrackInteraction(:final trackId) => _TrackPanel(
        trackId: trackId,
        onClose: onClose,
      ),
    };
  }
}

/// Panel for single clip interaction
class _ClipPanel extends StatelessWidget {
  final int trackId;
  final int clipId;
  final VoidCallback onClose;

  const _ClipPanel({
    required this.trackId,
    required this.clipId,
    required this.onClose,
  });

  @override
  Widget build(BuildContext context) {
    final state = context.watch<KarbeatState>();
    final track = state.tracks[trackId];
    final clip = track?.clips.where((c) => c.id == clipId).firstOrNull;

    if (clip == null) {
      return const SizedBox();
    }

    final isMidiClip = clip.source is UiClipSource_Midi;
    final patternId = isMidiClip
        ? (clip.source as UiClipSource_Midi).patternId
        : null;

    return _PanelContainer(
      onClose: onClose,
      title: clip.name,
      icon: isMidiClip ? Icons.piano : Icons.audio_file,
      children: [
        _InfoRow(label: 'Track', value: track?.name ?? 'Unknown'),
        _InfoRow(label: 'Type', value: isMidiClip ? 'MIDI Pattern' : 'Audio'),
        _InfoRow(
          label: 'Position',
          value: _formatSamples(
            clip.startTime.toInt(),
            state.hardwareConfig.sampleRate,
          ),
        ),
        const SizedBox(height: 16),
        _ActionGrid(
          actions: [
            _ActionItem(
              icon: Icons.edit,
              label: 'Rename',
              onTap: () => _showRenameDialog(context, clip.name),
            ),
            if (isMidiClip && patternId != null)
              _ActionItem(
                icon: Icons.piano,
                label: 'Edit Pattern',
                onTap: () {
                  onClose();
                  state.openPattern(patternId);
                },
              ),
            _ActionItem(
              icon: Icons.content_copy,
              label: 'Duplicate',
              onTap: () {
                // TODO: Implement duplicate
                onClose();
              },
            ),
            _ActionItem(
              icon: Icons.delete,
              label: 'Delete',
              isDestructive: true,
              onTap: () {
                state.deleteClip(trackId, clipId);
                onClose();
              },
            ),
          ],
        ),
      ],
    );
  }

  void _showRenameDialog(BuildContext context, String currentName) {
    // TODO: Implement rename dialog and API
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(const SnackBar(content: Text('Rename not yet implemented')));
  }

  String _formatSamples(int samples, int sampleRate) {
    if (sampleRate <= 0) return '$samples samples';
    final seconds = samples / sampleRate;
    final minutes = (seconds / 60).floor();
    final secs = (seconds % 60).toStringAsFixed(2);
    return '$minutes:${secs.padLeft(5, '0')}';
  }
}

/// Panel for multi-clip interaction
class _MultiClipPanel extends StatelessWidget {
  final int trackId;
  final List<int> clipIds;
  final VoidCallback onClose;

  const _MultiClipPanel({
    required this.trackId,
    required this.clipIds,
    required this.onClose,
  });

  @override
  Widget build(BuildContext context) {
    final state = context.watch<KarbeatState>();
    final track = state.tracks[trackId];

    return _PanelContainer(
      onClose: onClose,
      title: '${clipIds.length} Clips Selected',
      icon: Icons.select_all,
      children: [
        _InfoRow(label: 'Track', value: track?.name ?? 'Unknown'),
        _InfoRow(label: 'Count', value: '${clipIds.length} clips'),
        const SizedBox(height: 16),
        _ActionGrid(
          actions: [
            _ActionItem(
              icon: Icons.deselect,
              label: 'Deselect',
              onTap: () {
                state.deselectAllClips();
                onClose();
              },
            ),
            _ActionItem(
              icon: Icons.content_copy,
              label: 'Duplicate All',
              onTap: () {
                // TODO: Implement batch duplicate
                onClose();
              },
            ),
            _ActionItem(
              icon: Icons.delete,
              label: 'Delete All',
              isDestructive: true,
              onTap: () {
                state.deleteSelectedClips();
                onClose();
              },
            ),
          ],
        ),
      ],
    );
  }
}

/// Panel for track interaction
class _TrackPanel extends StatelessWidget {
  final int trackId;
  final VoidCallback onClose;

  const _TrackPanel({required this.trackId, required this.onClose});

  @override
  Widget build(BuildContext context) {
    final state = context.watch<KarbeatState>();
    final track = state.tracks[trackId];

    if (track == null) {
      return const SizedBox();
    }

    final isMidiTrack = track.trackType == TrackType.midi;

    return _PanelContainer(
      onClose: onClose,
      title: track.name,
      icon: isMidiTrack ? Icons.piano : Icons.audio_file,
      children: [
        _InfoRow(label: 'Type', value: isMidiTrack ? 'MIDI' : 'Audio'),
        _InfoRow(label: 'Clips', value: '${track.clips.length}'),
        const SizedBox(height: 16),
        _ActionGrid(
          actions: [
            _ActionItem(
              icon: Icons.edit,
              label: 'Rename',
              onTap: () => _showRenameDialog(context, track.name),
            ),
            if (isMidiTrack && track.generatorId != null)
              _ActionItem(
                icon: Icons.settings,
                label: 'Generator',
                onTap: () {
                  // TODO: Navigate to generator settings
                  onClose();
                },
              ),
            _ActionItem(
              icon: Icons.delete,
              label: 'Delete Track',
              isDestructive: true,
              onTap: () {
                // TODO: Implement delete track
                onClose();
              },
            ),
          ],
        ),
      ],
    );
  }

  void _showRenameDialog(BuildContext context, String currentName) {
    // TODO: Implement rename dialog
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(content: Text('Track rename not yet implemented')),
    );
  }
}

// =============================================================================
// SHARED UI COMPONENTS
// =============================================================================

class _PanelContainer extends StatelessWidget {
  final String title;
  final IconData icon;
  final List<Widget> children;
  final VoidCallback onClose;

  const _PanelContainer({
    required this.title,
    required this.icon,
    required this.children,
    required this.onClose,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      constraints: const BoxConstraints(maxWidth: 320),
      decoration: BoxDecoration(
        color: Colors.grey.shade900,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: Colors.white.withAlpha(30)),
        boxShadow: [
          BoxShadow(
            color: Colors.black.withAlpha(100),
            blurRadius: 20,
            offset: const Offset(0, 8),
          ),
        ],
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // Header
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
            decoration: BoxDecoration(
              color: Colors.cyanAccent.withAlpha(30),
              borderRadius: const BorderRadius.vertical(
                top: Radius.circular(12),
              ),
            ),
            child: Row(
              children: [
                Icon(icon, color: Colors.cyanAccent, size: 20),
                const SizedBox(width: 10),
                Expanded(
                  child: Text(
                    title,
                    style: const TextStyle(
                      color: Colors.white,
                      fontWeight: FontWeight.bold,
                      fontSize: 14,
                    ),
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
                IconButton(
                  icon: const Icon(
                    Icons.close,
                    color: Colors.white54,
                    size: 18,
                  ),
                  onPressed: onClose,
                  padding: EdgeInsets.zero,
                  constraints: const BoxConstraints(),
                ),
              ],
            ),
          ),
          // Content
          Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: children,
            ),
          ),
        ],
      ),
    );
  }
}

class _InfoRow extends StatelessWidget {
  final String label;
  final String value;

  const _InfoRow({required this.label, required this.value});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        children: [
          Text(
            '$label:',
            style: TextStyle(color: Colors.white.withAlpha(150), fontSize: 12),
          ),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              value,
              style: const TextStyle(color: Colors.white, fontSize: 12),
              textAlign: TextAlign.right,
            ),
          ),
        ],
      ),
    );
  }
}

class _ActionGrid extends StatelessWidget {
  final List<_ActionItem> actions;

  const _ActionGrid({required this.actions});

  @override
  Widget build(BuildContext context) {
    return Wrap(spacing: 8, runSpacing: 8, children: actions);
  }
}

class _ActionItem extends StatelessWidget {
  final IconData icon;
  final String label;
  final VoidCallback onTap;
  final bool isDestructive;

  const _ActionItem({
    required this.icon,
    required this.label,
    required this.onTap,
    this.isDestructive = false,
  });

  @override
  Widget build(BuildContext context) {
    final color = isDestructive ? Colors.redAccent : Colors.cyanAccent;

    return Material(
      color: Colors.transparent,
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(8),
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
          decoration: BoxDecoration(
            border: Border.all(color: color.withAlpha(80)),
            borderRadius: BorderRadius.circular(8),
          ),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(icon, size: 16, color: color),
              const SizedBox(width: 6),
              Text(label, style: TextStyle(color: color, fontSize: 12)),
            ],
          ),
        ),
      ),
    );
  }
}
