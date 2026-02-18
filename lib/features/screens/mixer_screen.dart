import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:karbeat/src/rust/api/mixer.dart';
import 'package:karbeat/state/app_state.dart';
import 'package:provider/provider.dart';

class MixerScreen extends StatefulWidget {
  const MixerScreen({super.key});

  @override
  State<MixerScreen> createState() => _MixerScreenState();
}

class _MixerScreenState extends State<MixerScreen> {
  @override
  void initState() {
    super.initState();
    // TODO: Integrate with backend – call syncMixerState() on init
    // and subscribe to mixer state changes.
    WidgetsBinding.instance.addPostFrameCallback((_) {
      context.read<KarbeatState>().syncMixerState();
    });
  }

  @override
  Widget build(BuildContext context) {
    final mixerState = context.select<KarbeatState, UiMixerState>(
      (state) => state.mixerState,
    );
    final tracks = context.select<KarbeatState, Map<int, dynamic>>(
      (state) => state.tracks,
    );

    // Channel entries: pair each track ID with its mixer channel data
    final channelEntries = <_ChannelEntry>[];
    for (final trackId in mixerState.channels.keys) {
      final channel = mixerState.channels[trackId]!;
      final trackName = tracks[trackId]?.name ?? 'Track $trackId';
      channelEntries.add(
        _ChannelEntry(
          id: trackId,
          name: trackName,
          channel: channel,
          isMaster: false,
        ),
      );
    }

    return Scaffold(
      backgroundColor: Colors.grey.shade900,
      body: Row(
        children: [
          // === Track Channels (scrollable) ===
          Expanded(
            child: channelEntries.isEmpty
                ? Center(
                    child: Text(
                      'No channels',
                      style: TextStyle(
                        color: Colors.grey.shade600,
                        fontStyle: FontStyle.italic,
                      ),
                    ),
                  )
                : ListView.builder(
                    scrollDirection: Axis.horizontal,
                    padding: const EdgeInsets.symmetric(
                      horizontal: 8,
                      vertical: 12,
                    ),
                    itemCount: channelEntries.length,
                    itemBuilder: (context, index) {
                      final entry = channelEntries[index];
                      return _ChannelStrip(
                        entry: entry,
                        onVolumeChanged: (value) {
                          // TODO: Integrate with backend
                          // setMixerChannelParams(
                          //   trackId: entry.id,
                          //   params: [UiMixerChannelParams.volume(value)],
                          // );
                          // context.read<KarbeatState>().syncMixerState();
                        },
                        onPanChanged: (value) {
                          // TODO: Integrate with backend
                          // setMixerChannelParams(
                          //   trackId: entry.id,
                          //   params: [UiMixerChannelParams.pan(value)],
                          // );
                        },
                        onMuteToggled: () {
                          // TODO: Integrate with backend
                          // setMixerChannelParams(
                          //   trackId: entry.id,
                          //   params: [UiMixerChannelParams.mute(!entry.channel.mute)],
                          // );
                        },
                        onSoloToggled: () {
                          // TODO: Integrate with backend – solo is not
                          // yet exposed via UiMixerChannelParams.
                        },
                      );
                    },
                  ),
          ),

          // === Divider ===
          Container(width: 1, color: Colors.white10),

          // === Master Channel (fixed) ===
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 12),
            child: _ChannelStrip(
              entry: _ChannelEntry(
                id: -1,
                name: 'Master',
                channel: mixerState.masterBus,
                isMaster: true,
              ),
              onVolumeChanged: (value) {
                // TODO: Integrate with backend
                // setMasterBusParams(
                //   params: [UiMixerChannelParams.volume(value)],
                // );
                // context.read<KarbeatState>().syncMixerState();
              },
              onPanChanged: (value) {
                // TODO: Integrate with backend
                // setMasterBusParams(
                //   params: [UiMixerChannelParams.pan(value)],
                // );
              },
              onMuteToggled: () {
                // TODO: Integrate with backend
                // setMasterBusParams(
                //   params: [UiMixerChannelParams.mute(!mixerState.masterBus.mute)],
                // );
              },
              onSoloToggled: () {
                // TODO: Integrate with backend
              },
            ),
          ),
        ],
      ),
    );
  }
}

// =========================================================
// Data helper
// =========================================================

class _ChannelEntry {
  final int id;
  final String name;
  final UiMixerChannel channel;
  final bool isMaster;

  const _ChannelEntry({
    required this.id,
    required this.name,
    required this.channel,
    required this.isMaster,
  });
}

// =========================================================
// Channel Strip Widget
// =========================================================

class _ChannelStrip extends StatelessWidget {
  final _ChannelEntry entry;
  final ValueChanged<double> onVolumeChanged;
  final ValueChanged<double> onPanChanged;
  final VoidCallback onMuteToggled;
  final VoidCallback onSoloToggled;

  const _ChannelStrip({
    required this.entry,
    required this.onVolumeChanged,
    required this.onPanChanged,
    required this.onMuteToggled,
    required this.onSoloToggled,
  });

  @override
  Widget build(BuildContext context) {
    final accentColor = entry.isMaster
        ? const Color(0xFFFFD700)
        : const Color(0xFF00E5FF);

    return Container(
      width: 72,
      margin: const EdgeInsets.symmetric(horizontal: 4),
      decoration: BoxDecoration(
        color: entry.isMaster
            ? const Color(0xFF2A2040)
            : const Color(0xFF16213E),
        borderRadius: BorderRadius.circular(10),
        border: Border.all(
          color: entry.isMaster
              ? Colors.amber.withValues(alpha: 0.3)
              : Colors.white.withValues(alpha: 0.06),
        ),
      ),
      child: Column(
        children: [
          // === Channel Label ===
          Container(
            width: double.infinity,
            padding: const EdgeInsets.symmetric(vertical: 8),
            decoration: BoxDecoration(
              color: accentColor.withValues(alpha: 0.15),
              borderRadius: const BorderRadius.vertical(
                top: Radius.circular(9),
              ),
            ),
            child: Text(
              entry.name,
              textAlign: TextAlign.center,
              overflow: TextOverflow.ellipsis,
              style: TextStyle(
                color: accentColor,
                fontSize: 11,
                fontWeight: FontWeight.w600,
                letterSpacing: 0.5,
              ),
            ),
          ),

          const SizedBox(height: 6),

          // === Pan Knob (placeholder rotary) ===
          _PanKnob(
            value: entry.channel.pan,
            accentColor: accentColor,
            onChanged: onPanChanged,
          ),

          const SizedBox(height: 4),

          // === Volume Fader ===
          Expanded(
            child: _VolumeFader(
              value: entry.channel.volume,
              accentColor: accentColor,
              onChanged: onVolumeChanged,
            ),
          ),

          const SizedBox(height: 4),

          // === dB readout ===
          Text(
            _volumeToDb(entry.channel.volume),
            style: TextStyle(
              color: Colors.grey.shade500,
              fontSize: 9,
              fontFamily: 'monospace',
            ),
          ),

          const SizedBox(height: 6),

          // === Mute / Solo ===
          Row(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              _ToggleButton(
                label: 'M',
                isActive: entry.channel.mute,
                activeColor: Colors.redAccent,
                onTap: onMuteToggled,
              ),
              const SizedBox(width: 4),
              _ToggleButton(
                label: 'S',
                isActive: entry.channel.solo,
                activeColor: Colors.amber,
                onTap: onSoloToggled,
              ),
            ],
          ),

          const SizedBox(height: 8),
        ],
      ),
    );
  }

  String _volumeToDb(double volume) {
    if (volume <= 0.0) return '-∞ dB';
    final db = 20 * (math.log(volume) / math.ln10);
    return '${db.toStringAsFixed(1)} dB';
  }
}

// =========================================================
// Pan Knob (simplified horizontal slider)
// =========================================================

class _PanKnob extends StatelessWidget {
  final double value; // -1.0 (L) to 1.0 (R)
  final Color accentColor;
  final ValueChanged<double> onChanged;

  const _PanKnob({
    required this.value,
    required this.accentColor,
    required this.onChanged,
  });

  @override
  Widget build(BuildContext context) {
    final label = value == 0
        ? 'C'
        : value < 0
        ? 'L${(-value * 100).round()}'
        : 'R${(value * 100).round()}';

    return Column(
      children: [
        Text(
          label,
          style: TextStyle(
            color: Colors.grey.shade500,
            fontSize: 9,
            fontFamily: 'monospace',
          ),
        ),
        const SizedBox(height: 2),
        SizedBox(
          width: 56,
          height: 20,
          child: SliderTheme(
            data: SliderThemeData(
              trackHeight: 3,
              thumbShape: const RoundSliderThumbShape(enabledThumbRadius: 5),
              activeTrackColor: accentColor,
              inactiveTrackColor: Colors.white12,
              thumbColor: accentColor,
              overlayShape: SliderComponentShape.noOverlay,
            ),
            child: Slider(
              value: value,
              min: -1.0,
              max: 1.0,
              onChanged: onChanged,
            ),
          ),
        ),
      ],
    );
  }
}

// =========================================================
// Volume Fader (vertical slider)
// =========================================================

class _VolumeFader extends StatelessWidget {
  final double value; // 0.0 to ~1.5 (or 2.0)
  final Color accentColor;
  final ValueChanged<double> onChanged;

  const _VolumeFader({
    required this.value,
    required this.accentColor,
    required this.onChanged,
  });

  @override
  Widget build(BuildContext context) {
    return RotatedBox(
      quarterTurns: 3,
      child: SliderTheme(
        data: SliderThemeData(
          trackHeight: 4,
          thumbShape: const RoundSliderThumbShape(enabledThumbRadius: 7),
          activeTrackColor: accentColor,
          inactiveTrackColor: Colors.white10,
          thumbColor: accentColor,
          overlayColor: accentColor.withValues(alpha: 0.15),
          overlayShape: const RoundSliderOverlayShape(overlayRadius: 12),
        ),
        child: Slider(
          value: value.clamp(0.0, 2.0),
          min: 0.0,
          max: 2.0,
          onChanged: onChanged,
        ),
      ),
    );
  }
}

// =========================================================
// Small Toggle Button (Mute / Solo)
// =========================================================

class _ToggleButton extends StatelessWidget {
  final String label;
  final bool isActive;
  final Color activeColor;
  final VoidCallback onTap;

  const _ToggleButton({
    required this.label,
    required this.isActive,
    required this.activeColor,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: onTap,
      child: Container(
        width: 26,
        height: 22,
        decoration: BoxDecoration(
          color: isActive
              ? activeColor.withValues(alpha: 0.85)
              : Colors.white.withValues(alpha: 0.06),
          borderRadius: BorderRadius.circular(4),
          border: Border.all(
            color: isActive
                ? activeColor
                : Colors.white.withValues(alpha: 0.12),
            width: 1,
          ),
        ),
        alignment: Alignment.center,
        child: Text(
          label,
          style: TextStyle(
            color: isActive ? Colors.black87 : Colors.grey.shade500,
            fontSize: 10,
            fontWeight: FontWeight.bold,
          ),
        ),
      ),
    );
  }
}
