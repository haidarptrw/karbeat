import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:karbeat/features/components/fine_grained_input.dart';
import 'package:karbeat/features/audio_plugins/effects/effect_registry.dart';
import 'package:karbeat/src/rust/api/mixer.dart';
import 'package:karbeat/src/rust/api/plugin.dart';
import 'package:karbeat/src/rust/api/plugin.dart' as plugin_api;
import 'package:karbeat/state/app_state.dart';
import 'package:karbeat/utils/logger.dart';

class MixerScreen extends ConsumerStatefulWidget {
  const MixerScreen({super.key});

  @override
  ConsumerState<MixerScreen> createState() => _MixerScreenState();
}

class _MixerScreenState extends ConsumerState<MixerScreen> {
  // Track the currently selected channel ID (or -1 for Master)
  int? _selectedChannelId;
  bool _isSelectedBus = false;

  // Initial width for the track panel
  double _trackPanelWidth = 400.0;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      ref.read(karbeatStateProvider).syncMixerState();
    });
  }

  @override
  Widget build(BuildContext context) {
    final mixerState = ref.watch(
      karbeatStateProvider.select((s) => s.mixerState),
    );
    final tracks = ref.watch(karbeatStateProvider.select((s) => s.tracks));

    final state = ref.read(karbeatStateProvider);

    // Channel entries: pair each track ID with its mixer channel data
    final channelEntries = <_ChannelEntry>[];

    final sortedTrackIds = mixerState.channels.keys.toList()..sort();
    for (final trackId in sortedTrackIds) {
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

    final busEntries = <_ChannelEntry>[];
    for (final bus in mixerState.buses.values) {
      busEntries.add(
        _ChannelEntry(
          id: bus.id,
          name: bus.name,
          channel: bus.channel,
          isMaster: false,
          isBus: true,
        ),
      );
    }

    return Scaffold(
      backgroundColor: Colors.grey.shade900,
      body: LayoutBuilder(
        builder: (context, constraints) {
          // Safety bounds for resizing (e.g., don't let a panel get smaller than 150px)
          const double minPanelWidth = 150.0;
          // Leave room for the bus panel and the master bus
          final double maxPanelWidth =
              constraints.maxWidth - minPanelWidth - 100.0;

          // Ensure our starting width is within bounds
          _trackPanelWidth = _trackPanelWidth.clamp(
            minPanelWidth,
            maxPanelWidth,
          );

          return Row(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              // === Track Channels (scrollable, fixed width) ===
              SizedBox(
                width: _trackPanelWidth,
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
                          return KeyedSubtree(
                            key: ValueKey('mixer_track_${entry.id}'),
                            child: _ChannelStrip(
                              entry: entry,
                              onVolumeChanged: (value) {
                                state.setMixerChannelParams(
                                  trackId: entry.id,
                                  params: [UiMixerChannelParams.volume(value)],
                                );
                              },
                              onVolumeChangeStart: () {
                                state.markParamTouched(entry.id, 'volume');
                              },
                              onVolumeChangeEnd: () {
                                state.markParamReleased(entry.id, 'volume');
                              },
                              onPanChanged: (value) {
                                state.setMixerChannelParams(
                                  trackId: entry.id,
                                  params: [UiMixerChannelParams.pan(value)],
                                );
                              },
                              onPanChangeStart: () {
                                state.markParamTouched(entry.id, 'pan');
                              },
                              onPanChangeEnd: () {
                                state.markParamReleased(entry.id, 'pan');
                              },
                              onMuteToggled: () {
                                state.setMixerChannelParams(
                                  trackId: entry.id,
                                  params: [
                                    UiMixerChannelParams.mute(
                                      !entry.channel.mute,
                                    ),
                                  ],
                                );
                              },
                              onSoloToggled: () {
                                state.setMixerChannelParams(
                                  trackId: entry.id,
                                  params: [
                                    UiMixerChannelParams.solo(
                                      !entry.channel.solo,
                                    ),
                                  ],
                                );
                              },
                              isSelected:
                                  _selectedChannelId == entry.id &&
                                  !_isSelectedBus &&
                                  !entry.isMaster,
                              onTap: () {
                                setState(() {
                                  _selectedChannelId = entry.id;
                                  _isSelectedBus = false;
                                });
                              },
                            ),
                          );
                        },
                      ),
              ),

              // === Resizable Divider between Tracks and Buses ===
              GestureDetector(
                onHorizontalDragUpdate: (details) {
                  setState(() {
                    _trackPanelWidth = (_trackPanelWidth + details.delta.dx)
                        .clamp(minPanelWidth, maxPanelWidth);
                  });
                },
                child: MouseRegion(
                  cursor: SystemMouseCursors.resizeColumn,
                  child: Container(
                    width: 8,
                    color: Colors.transparent,
                    child: Center(
                      child: Container(
                        width: 2,
                        height: 40,
                        decoration: BoxDecoration(
                          color: Colors.white24,
                          borderRadius: BorderRadius.circular(1),
                        ),
                      ),
                    ),
                  ),
                ),
              ),

              // === Bus Channels (scrollable) ===
              Expanded(
                child: ListView.builder(
                  scrollDirection: Axis.horizontal,
                  padding: const EdgeInsets.symmetric(
                    horizontal: 8,
                    vertical: 12,
                  ),
                  itemCount: busEntries.length + 1,
                  itemBuilder: (context, index) {
                    // Last item: "Add Bus" ghost strip
                    if (index == busEntries.length) {
                      return GestureDetector(
                        onTap: () async {
                          final busCount = busEntries.length + 1;
                          await state.createNewBusChannel(
                            name: "Bus $busCount",
                          );
                        },
                        child: Container(
                          width: 72,
                          margin: const EdgeInsets.symmetric(horizontal: 4),
                          decoration: BoxDecoration(
                            color: Colors.white.withValues(alpha: 0.02),
                            borderRadius: BorderRadius.circular(10),
                            border: Border.all(
                              color: Colors.white.withValues(alpha: 0.08),
                              width: 1,
                            ),
                          ),
                          child: Column(
                            mainAxisAlignment: MainAxisAlignment.center,
                            children: [
                              Icon(
                                Icons.add_rounded,
                                color: Colors.white.withValues(alpha: 0.25),
                                size: 28,
                              ),
                              const SizedBox(height: 6),
                              Text(
                                'Add Bus',
                                style: TextStyle(
                                  color: Colors.white.withValues(alpha: 0.25),
                                  fontSize: 10,
                                  fontWeight: FontWeight.w500,
                                ),
                              ),
                            ],
                          ),
                        ),
                      );
                    }

                    final entry = busEntries[index];
                    return KeyedSubtree(
                      key: ValueKey('mixer_bus_${entry.id}'),
                      child: _ChannelStrip(
                        entry: entry,
                        onVolumeChanged: (value) {
                          state.setBusChannelParams(
                            busId: entry.id,
                            params: [UiMixerChannelParams.volume(value)],
                          );
                        },
                        onVolumeChangeStart: () {
                          state.markParamTouched(entry.id, 'volume');
                        },
                        onVolumeChangeEnd: () {
                          state.markParamReleased(entry.id, 'volume');
                        },
                        onPanChanged: (value) {
                          state.setBusChannelParams(
                            busId: entry.id,
                            params: [UiMixerChannelParams.pan(value)],
                          );
                        },
                        onPanChangeStart: () {
                          state.markParamTouched(entry.id, 'pan');
                        },
                        onPanChangeEnd: () {
                          state.markParamReleased(entry.id, 'pan');
                        },
                        onMuteToggled: () {
                          state.setBusChannelParams(
                            busId: entry.id,
                            params: [
                              UiMixerChannelParams.mute(!entry.channel.mute),
                            ],
                          );
                        },
                        onSoloToggled: () {
                          state.setBusChannelParams(
                            busId: entry.id,
                            params: [
                              UiMixerChannelParams.solo(!entry.channel.solo),
                            ],
                          );
                        },
                        isSelected:
                            _selectedChannelId == entry.id && _isSelectedBus,
                        onTap: () {
                          setState(() {
                            _selectedChannelId = entry.id;
                            _isSelectedBus = true;
                          });
                        },
                      ),
                    );
                  },
                ),
              ),

              // === Divider ===
              Container(width: 1, color: Colors.white10),

              // === Master Channel (fixed) ===
              Padding(
                padding: const EdgeInsets.symmetric(
                  horizontal: 4,
                  vertical: 12,
                ),
                child: _ChannelStrip(
                  entry: _ChannelEntry(
                    id: -1,
                    name: 'Master',
                    channel: mixerState.masterBus,
                    isMaster: true,
                  ),
                  onVolumeChanged: (value) {
                    state.setMasterBusParams(
                      params: [UiMixerChannelParams.volume(value)],
                    );
                  },
                  onVolumeChangeStart: () {
                    // u32::MAX for master bus
                    state.markParamTouched(4294967295, 'volume');
                  },
                  onVolumeChangeEnd: () {
                    state.markParamReleased(4294967295, 'volume');
                  },
                  onPanChanged: (value) {
                    state.setMasterBusParams(
                      params: [UiMixerChannelParams.pan(value)],
                    );
                  },
                  onPanChangeStart: () {
                    state.markParamTouched(4294967295, 'pan');
                  },
                  onPanChangeEnd: () {
                    state.markParamReleased(4294967295, 'pan');
                  },
                  onMuteToggled: () {
                    state.setMasterBusParams(
                      params: [
                        UiMixerChannelParams.mute(!mixerState.masterBus.mute),
                      ],
                    );
                  },
                  onSoloToggled: () {
                    state.setMasterBusParams(
                      params: [
                        UiMixerChannelParams.solo(!mixerState.masterBus.solo),
                      ],
                    );
                  },
                  isSelected: _selectedChannelId == -1 && !_isSelectedBus,
                  onTap: () {
                    setState(() {
                      _selectedChannelId = -1;
                      _isSelectedBus = false;
                    });
                  },
                ),
              ),

              // === Divider ===
              Container(width: 1, color: Colors.white10),

              // === Effect Rack Panel ===
              _buildEffectRackPanel(context, mixerState),
            ],
          );
        },
      ),
    );
  }

  Widget _buildEffectRackPanel(BuildContext ctx, UiMixerState mixerState) {
    if (_selectedChannelId == null) {
      return const SizedBox(
        width: 250,
        child: Center(
          child: Text(
            'Select a channel to\nview effects',
            textAlign: TextAlign.center,
            style: TextStyle(color: Colors.white54, fontSize: 13),
          ),
        ),
      );
    }

    final isMaster = _selectedChannelId == -1 && !_isSelectedBus;
    final channel = isMaster
        ? mixerState.masterBus
        : _isSelectedBus
        ? mixerState.buses[_selectedChannelId!]?.channel
        : mixerState.channels[_selectedChannelId!];

    if (channel == null) {
      return const SizedBox(width: 250);
    }

    final channelName = isMaster
        ? 'Master'
        : (_isSelectedBus
              ? 'Bus $_selectedChannelId'
              : 'Track $_selectedChannelId');

    return Container(
      width: 250,
      color: Colors.grey.shade900,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // Header
          Container(
            padding: const EdgeInsets.symmetric(vertical: 12, horizontal: 16),
            color: Colors.black26,
            child: Row(
              children: [
                const Icon(Icons.blur_on, color: Colors.white70, size: 18),
                const SizedBox(width: 8),
                Text(
                  '$channelName Effects',
                  style: const TextStyle(
                    color: Colors.white,
                    fontWeight: FontWeight.bold,
                    backgroundColor: Colors.transparent,
                    fontSize: 14,
                  ),
                ),
              ],
            ),
          ),

          // Effects List
          Expanded(
            child: channel.effects.isEmpty
                ? const Center(
                    child: Text(
                      'No effects',
                      style: TextStyle(
                        color: Colors.white38,
                        fontStyle: FontStyle.italic,
                      ),
                    ),
                  )
                : ListView.builder(
                    padding: const EdgeInsets.all(8),
                    itemCount: channel.effects.length,
                    itemBuilder: (context, index) {
                      final effect = channel.effects[index];
                      return Container(
                        margin: const EdgeInsets.only(bottom: 8),
                        decoration: BoxDecoration(
                          color: Colors.white.withAlpha(10),
                          borderRadius: BorderRadius.circular(6),
                          border: Border.all(color: Colors.white.withAlpha(20)),
                        ),
                        child: ListTile(
                          dense: true,
                          title: Text(
                            effect.name,
                            style: const TextStyle(color: Colors.white),
                          ),
                          subtitle: Text(
                            'ID: ${effect.id}',
                            style: const TextStyle(color: Colors.white54),
                          ),
                          trailing: const Icon(
                            Icons.settings,
                            color: Colors.white54,
                            size: 16,
                          ),
                          onTap: () {
                            try {
                              final target = isMaster
                                  ? const plugin_api.UiEffectTarget.master()
                                  : _isSelectedBus
                                  ? plugin_api.UiEffectTarget.bus(
                                      _selectedChannelId!,
                                    )
                                  : plugin_api.UiEffectTarget.track(
                                      _selectedChannelId!,
                                    );

                              final availableEffects = ref
                                  .read(karbeatStateProvider)
                                  .availableEffects;
                              final registryId = availableEffects
                                  .firstWhere((p) => p.id == effect.registryId)
                                  .id;
                              final builder = EffectRegistry.getEffectBuilder(
                                registryId,
                              );
                              final screen = builder(effect.id, target);

                              Navigator.push(
                                context,
                                MaterialPageRoute(builder: (context) => screen),
                              );
                            } catch (_) {
                              // Feedback for effects that don't have a UI yet
                              ScaffoldMessenger.of(context).showSnackBar(
                                SnackBar(
                                  content: Text(
                                    '${effect.name} UI is not implemented yet.',
                                  ),
                                  duration: const Duration(seconds: 2),
                                ),
                              );
                            }
                          },
                        ),
                      );
                    },
                  ),
          ),

          // Add Effect Button
          Padding(
            padding: const EdgeInsets.all(8.0),
            child: ElevatedButton.icon(
              style: ElevatedButton.styleFrom(
                backgroundColor: Colors.white.withAlpha(20),
                foregroundColor: Colors.white,
              ),
              onPressed: () {
                _showEffectBrowser(context);
              },
              icon: const Icon(Icons.add, size: 16),
              label: const Text('Add Effect'),
            ),
          ),
        ],
      ),
    );
  }

  void _showEffectBrowser(BuildContext context) {
    final availablePlugins = ref.read(karbeatStateProvider).availableEffects;

    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text("Generator Browser"),
        contentPadding: const EdgeInsets.only(top: 12, bottom: 24),
        content: SizedBox(
          width: 360,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // Category header: Karbeat Native
              Padding(
                padding: const EdgeInsets.symmetric(
                  horizontal: 24,
                  vertical: 8,
                ),
                child: Row(
                  children: [
                    Icon(
                      Icons.extension,
                      size: 16,
                      color: Colors.deepOrangeAccent,
                    ),
                    const SizedBox(width: 8),
                    Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 8,
                        vertical: 3,
                      ),
                      decoration: BoxDecoration(
                        color: Colors.deepOrangeAccent.withAlpha(30),
                        borderRadius: BorderRadius.circular(4),
                        border: Border.all(
                          color: Colors.deepOrangeAccent.withAlpha(80),
                        ),
                      ),
                      child: const Text(
                        "Karbeat Native",
                        style: TextStyle(
                          color: Colors.deepOrangeAccent,
                          fontSize: 12,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                  ],
                ),
              ),
              const Divider(height: 1),
              // Plugin list
              if (availablePlugins.isEmpty)
                const Padding(
                  padding: EdgeInsets.symmetric(horizontal: 24, vertical: 16),
                  child: Text(
                    "No effects found",
                    style: TextStyle(color: Colors.grey),
                  ),
                )
              else
                ...availablePlugins.map(
                  (plugin) => _buildEffectBrowserItem(ctx, plugin),
                ),
            ],
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: const Text("Cancel"),
          ),
        ],
      ),
    );
  }

  Widget _buildEffectBrowserItem(BuildContext ctx, UiPluginInfo plugin) {
    return InkWell(
      onTap: () {
        Navigator.pop(ctx);
        if (plugin.pluginType != KarbeatPluginType.effect) {
          ScaffoldMessenger.of(ctx).showSnackBar(
            const SnackBar(
              content: Text(
                'Only effects can be added from the mixer panel for now.',
              ),
            ),
          );
          return;
        }
        if (_selectedChannelId == null) {
          ScaffoldMessenger.of(ctx).showSnackBar(
            const SnackBar(
              content: Text(
                'No channel selected. Please select a channel before adding an effect.',
              ),
            ),
          );
          return;
        }

        if (_selectedChannelId == -1 && !_isSelectedBus) {
          ref.read(karbeatStateProvider).addEffectToMasterBus(plugin.id);
          return;
        }

        if (_isSelectedBus) {
          ref
              .read(karbeatStateProvider)
              .addEffectToBusChannel(_selectedChannelId!, plugin.id);
        } else {
          ref
              .read(karbeatStateProvider)
              .addEffectToMixerChannel(_selectedChannelId!, plugin.id);
        }
      },
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 10),
        child: Row(
          children: [
            const Icon(Icons.piano, color: Colors.orangeAccent, size: 20),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    plugin.name,
                    style: const TextStyle(
                      fontSize: 14,
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                  const SizedBox(height: 2),
                  const Text(
                    "Karbeat Native",
                    style: TextStyle(fontSize: 11, color: Colors.grey),
                  ),
                ],
              ),
            ),
          ],
        ),
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
  final bool isBus;

  const _ChannelEntry({
    required this.id,
    required this.name,
    required this.channel,
    required this.isMaster,
    this.isBus = false,
  });
}

// =========================================================
// Channel Strip Widget
// =========================================================

class _ChannelStrip extends StatefulWidget {
  final _ChannelEntry entry;
  final ValueChanged<double> onVolumeChanged;
  final VoidCallback? onVolumeChangeStart;
  final VoidCallback? onVolumeChangeEnd;
  final ValueChanged<double> onPanChanged;
  final VoidCallback? onPanChangeStart;
  final VoidCallback? onPanChangeEnd;
  final VoidCallback onMuteToggled;
  final VoidCallback onSoloToggled;
  final bool isSelected;
  final VoidCallback onTap;

  const _ChannelStrip({
    required this.entry,
    required this.onVolumeChanged,
    this.onVolumeChangeStart,
    this.onVolumeChangeEnd,
    required this.onPanChanged,
    this.onPanChangeStart,
    this.onPanChangeEnd,
    required this.onMuteToggled,
    required this.onSoloToggled,
    required this.isSelected,
    required this.onTap,
  });

  @override
  State<_ChannelStrip> createState() => _ChannelStripState();
}

class _ChannelStripState extends State<_ChannelStrip> {
  List<ParameterSpecDTO>? _specs;

  @override
  void initState() {
    super.initState();
    _loadSpecs();
  }

  Future<void> _loadSpecs() async {
    List<ParameterSpecDTO>? fetchedSpecs;
    try {
      if (widget.entry.isMaster) {
        fetchedSpecs = await getMasterChannelSpecs();
      } else if (widget.entry.isBus) {
        fetchedSpecs = await getBusMixerChannelSpecs(busId: widget.entry.id);
      } else {
        fetchedSpecs = await getTrackMixerChannelSpecs(trackId: widget.entry.id);
      }
    } catch (e) {
      debugPrint("Failed to load channel specs: $e");
    }

    if (mounted) {
      setState(() {
        _specs = fetchedSpecs;
      });
    }
  }

  // Safe fallback spec generators just in case the Future hasn't resolved yet 
  // (Prevents the UI from glitching or throwing layout errors during the microsecond load)
  ParameterSpecDTO _getVolumeSpec() {
    if (_specs != null) {
      return _specs!.firstWhere((s) => s.id == 1, orElse: () => _defaultVolumeSpec());
    }
    return _defaultVolumeSpec();
  }

  ParameterSpecDTO _getPanSpec() {
    if (_specs != null) {
      return _specs!.firstWhere((s) => s.id == 2, orElse: () => _defaultPanSpec());
    }
    return _defaultPanSpec();
  }

  ParameterSpecDTO _defaultVolumeSpec() => const ParameterSpecDTO(
        id: 1, name: 'Volume', group: 'MixerChannel', value: 0.0,
        min: -100.0, max: 6.0, defaultValue: 0.0, step: 0.1,
        valueType: ParameterValueTypeDTO.float, choices: [],
      );

  ParameterSpecDTO _defaultPanSpec() => const ParameterSpecDTO(
        id: 2, name: 'Pan', group: 'MixerChannel', value: 0.0,
        min: -1.0, max: 1.0, defaultValue: 0.0, step: 0.01,
        valueType: ParameterValueTypeDTO.float, choices: [],
      );

  @override
  Widget build(BuildContext context) {
    final entry = widget.entry;
    final accentColor = entry.isMaster
        ? const Color(0xFFFFD700)
        : const Color(0xFF00E5FF);

    return GestureDetector(
      onTap: widget.onTap,
      child: Container(
        width: 72,
        margin: const EdgeInsets.symmetric(horizontal: 4),
        decoration: BoxDecoration(
          color: entry.isMaster
              ? const Color(0xFF2A2040)
              : const Color(0xFF16213E),
          borderRadius: BorderRadius.circular(10),
          border: Border.all(
            color: widget.isSelected
                ? accentColor
                : (entry.isMaster
                    ? Colors.amber.withValues(alpha: 0.3)
                    : Colors.white.withValues(alpha: 0.06)),
            width: widget.isSelected ? 2 : 1,
          ),
          boxShadow: widget.isSelected
              ? [
                  BoxShadow(
                    color: accentColor.withValues(alpha: 0.2),
                    blurRadius: 8,
                    spreadRadius: 1,
                  ),
                ]
              : null,
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

            // === Pan Knob ===
            _PanKnob(
              value: entry.channel.pan,
              spec: _getPanSpec(), // Pass the bound FFI Spec
              accentColor: accentColor,
              onChanged: widget.onPanChanged,
              onChangeStart: widget.onPanChangeStart,
              onChangeEnd: widget.onPanChangeEnd,
            ),

            const SizedBox(height: 4),

            // === Volume Fader ===
            Expanded(
              child: _VolumeFader(
                value: entry.channel.volume,
                spec: _getVolumeSpec(), // Pass the bound FFI Spec
                accentColor: accentColor,
                onChanged: widget.onVolumeChanged,
                onChangeStart: widget.onVolumeChangeStart,
                onChangeEnd: widget.onVolumeChangeEnd,
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
                  onTap: widget.onMuteToggled,
                ),
                const SizedBox(width: 4),
                _ToggleButton(
                  label: 'S',
                  isActive: entry.channel.solo,
                  activeColor: Colors.amber,
                  onTap: widget.onSoloToggled,
                ),
              ],
            ),

            const SizedBox(height: 8),
          ],
        ),
      ),
    );
  }

  String _volumeToDb(double volumeDb) {
    if (volumeDb <= -60.0) return '-∞ dB';
    return '${volumeDb.toStringAsFixed(1)} dB';
  }
}

// =========================================================
// Pan Knob
// =========================================================

class _PanKnob extends StatelessWidget {
  final double value;
  final ParameterSpecDTO spec; // Injected FFI Spec
  final Color accentColor;
  final ValueChanged<double> onChanged;
  final VoidCallback? onChangeStart;
  final VoidCallback? onChangeEnd;

  const _PanKnob({
    required this.value,
    required this.spec,
    required this.accentColor,
    required this.onChanged,
    this.onChangeStart,
    this.onChangeEnd,
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
            child: ParameterInteractionWrapper<double>(
              parameterName: spec.name,
              value: value,
              defaultValue: spec.defaultValue,
              min: spec.min,
              max: spec.max,
              step: spec.step == 0.0 ? 0.01 : spec.step, // Safe fallback step
              onChanged: onChanged,
              onAddAutomation: () {
                KarbeatLogger.info("Create automation for ${spec.name} (ID: ${spec.id})");
                // TODO: Dispatch to state to create the lane
              },
              child: Slider(
                value: value,
                min: spec.min,
                max: spec.max,
                onChanged: onChanged,
                allowedInteraction: SliderInteraction.slideOnly,
                onChangeStart: onChangeStart != null
                    ? (_) => onChangeStart!()
                    : null,
                onChangeEnd: onChangeEnd != null ? (_) => onChangeEnd!() : null,
              ),
            ),
          ),
        ),
      ],
    );
  }
}

// =========================================================
// Volume Fader
// =========================================================

class _VolumeFader extends StatelessWidget {
  final double value; 
  final ParameterSpecDTO spec;
  final Color accentColor;
  final ValueChanged<double> onChanged;
  final VoidCallback? onChangeStart;
  final VoidCallback? onChangeEnd;

  const _VolumeFader({
    required this.value,
    required this.spec,
    required this.accentColor,
    required this.onChanged,
    this.onChangeStart,
    this.onChangeEnd,
  });

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final sliderWidth = constraints.maxHeight;
        
        // Ensure the visual slider stops at -60dB even if the internal `NEG_INFINITY` is lower
        final visualMin = spec.min < -60.0 ? -60.0 : spec.min;

        return RotatedBox(
          quarterTurns: 3,
          child: ParameterInteractionWrapper<double>(
            parameterName: spec.name,
            value: value,
            defaultValue: spec.defaultValue,
            min: visualMin, 
            max: spec.max,
            step: spec.step == 0.0 ? 0.1 : spec.step, 
            onChanged: onChanged,
            onAddAutomation: () {
              debugPrint("Create automation for ${spec.name} (ID: ${spec.id})");
            },
            child: SizedBox(
              width: sliderWidth,
              height: constraints.maxWidth,
              child: SliderTheme(
                data: SliderThemeData(
                  trackHeight: 4,
                  thumbShape: const RoundSliderThumbShape(
                    enabledThumbRadius: 7,
                  ),
                  activeTrackColor: accentColor,
                  inactiveTrackColor: Colors.white10,
                  thumbColor: accentColor,
                  overlayColor: accentColor.withValues(alpha: 0.15),
                  overlayShape: const RoundSliderOverlayShape(
                    overlayRadius: 12,
                  ),
                ),
                child: Slider(
                  value: value.clamp(visualMin, spec.max),
                  min: visualMin,
                  max: spec.max,
                  onChanged: onChanged,
                  allowedInteraction: SliderInteraction.slideThumb,
                  onChangeStart: onChangeStart != null
                      ? (_) => onChangeStart!()
                      : null,
                  onChangeEnd: onChangeEnd != null
                      ? (_) => onChangeEnd!()
                      : null,
                ),
              ),
            ),
          ),
        );
      },
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