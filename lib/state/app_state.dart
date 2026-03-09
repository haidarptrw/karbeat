import 'dart:async';
import 'dart:developer';

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:karbeat/models/grid.dart';
import 'package:karbeat/models/interaction_target.dart';
import 'package:karbeat/models/menu_group.dart';
import 'package:karbeat/utils/result_type.dart';
import 'package:karbeat/src/rust/api/audio.dart';
import 'package:karbeat/src/rust/api/mixer.dart' as mixer_api;
import 'package:karbeat/src/rust/api/pattern.dart';
import 'package:karbeat/src/rust/api/plugin.dart';
import 'package:karbeat/src/rust/api/project.dart';
import 'package:karbeat/src/rust/api/track.dart';
import 'package:karbeat/src/rust/api/track.dart' as track_api;
import 'package:karbeat/src/rust/api/transport.dart' as transport_api;
import 'package:karbeat/src/rust/audio/event.dart';
import 'package:karbeat/src/rust/core/project.dart';

import 'package:karbeat/src/rust/core/project/track.dart';
import 'package:karbeat/src/rust/core/project/transport.dart';
import 'package:karbeat/utils/formatter.dart';
import 'package:karbeat/utils/logger.dart';

/// Top-level Riverpod provider for the app state
final karbeatStateProvider = ChangeNotifierProvider<KarbeatState>((ref) {
  return KarbeatState();
});

enum ToolSelection { pointer, cut, draw, move, delete, scrub, zoom, select }

/// Piano roll specific tool selection (independent from main toolbar)
enum PianoRollToolSelection { pointer, draw, delete, select, slice }

enum WorkspaceView { trackList, pianoRoll, mixer, source }

enum ToolbarMenuContextGroup { none, project, edit, view }

/// Events that trigger a state refresh
enum ProjectEvent {
  tracksChanged,
  transportChanged,
  metadataChanged,
  sourceListChanged,
  generatorListChanged,
  effectListChanged,
  configChanged,
  patternChanged,
  mixerChanged,
}

class KarbeatState extends ChangeNotifier {
  // ================== BACKEND STATES =========================
  TransportState _transportState = TransportState();

  // ProjectMetadata _metadata = ProjectMetadata(
  //   name: "Untitled",
  //   author: "User",
  //   version: "1.0.0",
  //   createdAt: 0, // Assuming u64
  // );
  ProjectMetadata _metadata = ProjectMetadata();

  AudioHardwareConfig _hardwareConfig = AudioHardwareConfig();

  mixer_api.UiMixerState _mixerState = mixer_api.UiMixerState();
  // List<Clipboard>

  // =================== STORES ==========================
  Map<int, UiTrack> _tracks = {};
  Map<int, AudioWaveformUiForAudioProperties> _audioSources = {};
  Map<int, UiGeneratorInstance> _generators = {};
  Map<int, UiPattern> _patterns = {};

  List<UiPluginInfo> _availableGenerators = [];
  List<UiPluginInfo> get availableGenerators => _availableGenerators;

  List<UiPluginInfo> _availableEffects = [];
  List<UiPluginInfo> get availableEffects => _availableEffects;

  static final List<KarbeatToolbarMenuGroup> menuGroups = [
    KarbeatToolbarMenuGroupFactory.createProjectMenuGroup(),
    KarbeatToolbarMenuGroupFactory.createEditMenuGroup(),
    KarbeatToolbarMenuGroupFactory.createViewMenuGroup(),
  ];

  int maxSamplesIndex = 2000;

  late final Stream<PlaybackPosition> _positionBroadcastStream;

  // STRATEGY: Internal Event Bus for State Synchronization
  final StreamController<ProjectEvent> _stateEventController =
      StreamController.broadcast();

  // A custom defined internal event bus for state synchronization
  final StreamController<Future<void> Function()> _customStateEventController =
      StreamController.broadcast();

  // ignore:unused_field
  StreamSubscription<ProjectEvent>? _stateSubscription;

  // Mixer event stream from Rust for automation/backend-initiated changes
  StreamSubscription<mixer_api.MixerParamEvent>? _mixerEventSubscription;

  /// Params currently being touched by the user (trackId, paramName).
  /// Automation events for these params are ignored while touched.
  final Set<(int, String)> _touchedParams = {};

  // =========== EDITOR STATE ====================
  ToolSelection _selectedTool = ToolSelection.pointer;
  WorkspaceView _currentView = WorkspaceView.trackList;
  ToolbarMenuContextGroup _currentToolbarContext = ToolbarMenuContextGroup.none;
  int _piannoRollGridDenom = 4;
  int? _editingPatternId;

  // =========== SESSION STATE (frontend-only) ====================
  int? _selectedTrackId;
  List<int> _selectedClipIds = [];
  int? _focusClipId;

  // =========== PIANO ROLL STATE ====================
  PianoRollToolSelection _pianoRollTool = PianoRollToolSelection.pointer;
  Set<int> _selectedNoteIds = {};
  int? _previewGeneratorId;

  /// Currently active interaction target for the interaction panel
  InteractionTarget? _interactionTarget;

  /// Denominator of the grid size (e.g 4 = 1/4 note, 16 = 1/16 note)
  int gridSize = 4;

  // ================== OTHER STATES ====================
  bool _pendingPlayRequest = false;

  // ================ CONSTRUCTOR ==================
  KarbeatState() {
    _positionBroadcastStream = createPositionStream().asBroadcastStream();
    _initStateListener();
    _positionBroadcastStream.listen((pos) {
      if (pos.isPlaying) {
        _pendingPlayRequest = false;
      }

      // Track pattern mode state from transport
      if (pos.isPatternMode != _transportState.isPatternPlaying) {
        _transportState = _transportState.copyWith(
          isPatternPlaying: pos.isPatternMode,
        );
        notifyListeners();
      }

      // Only react if the state has actually changed
      if (pos.isPlaying != _transportState.isPlaying) {
        if (_pendingPlayRequest && !pos.isPlaying) {
          return;
        }

        // Update UI
        _transportState = _transportState.copyWith(isPlaying: pos.isPlaying);
        notifyListeners();

        if (!pos.isPlaying) {
          transport_api.setPlaying(val: false);
        }
      }
    });
    syncTrackState();
    syncMaxSampleIndex();
    syncTransportState();
    syncMetadataState();
    syncAudioSourceList();
    syncPatternList();
    syncGeneratorList();
    syncAudioHardwareConfigState();

    // fetch available generators and effects
    fetchAvailableGenerators();
    fetchAvailableEffects();

    // Start mixer event stream
    _initMixerEventStream();
  }

  @override
  void dispose() {
    // Cancel active stream subscriptions from the Rust backend and internal event bus
    _stateSubscription?.cancel();
    _mixerEventSubscription?.cancel();

    // Close the internal event bus controllers
    if (!_stateEventController.isClosed) {
      _stateEventController.close();
    }

    if (!_customStateEventController.isClosed) {
      _customStateEventController.close();
    }

    // Always call super.dispose() last to properly tear down the ChangeNotifier
    super.dispose();
  }

  // =========================================================
  // ============= Available Plugins API =====================
  // =========================================================

  /// Fetch available generators from system's registry
  Future<void> fetchAvailableGenerators() async {
    try {
      // Use the ID-based API that returns UiPluginInfo with id and name
      final list = await getAvailableGeneratorsWithIds();
      _availableGenerators = list;
      notifyListeners();
    } catch (e) {
      log("Error fetching plugins: $e");
    }
  }

  /// Fetch available effects from system's registry
  Future<void> fetchAvailableEffects() async {
    try {
      final list = await getAvailableEffectsWithIds();
      _availableEffects = list;
      notifyListeners();
    } catch (e) {
      KarbeatLogger.error("Error fetching effect plugins: $e");
    }
  }

  void _initStateListener() {
    _stateSubscription = _stateEventController.stream.listen((event) async {
      switch (event) {
        case ProjectEvent.tracksChanged:
          await syncTrackState();
          await syncMaxSampleIndex();
          break;
        case ProjectEvent.transportChanged:
          await syncTransportState();
          break;
        case ProjectEvent.metadataChanged:
          await syncMetadataState();
          break;
        case ProjectEvent.sourceListChanged:
          await syncAudioSourceList();
          break;
        case ProjectEvent.generatorListChanged:
          await syncGeneratorList();
          break;
        case ProjectEvent.configChanged:
          await syncAudioHardwareConfigState();
          break;

        case ProjectEvent.patternChanged:
          await syncPatternList();
          break;
        case ProjectEvent.mixerChanged:
          await syncMixerState();
          break;
        case ProjectEvent.effectListChanged:
          // TODO: Handle this case.
          // throw UnimplementedError();
          break;
      }
    });

    // Listen and execute caller-defined custom sync actions
    _customStateEventController.stream.listen((action) async {
      try {
        await action();
      } catch (e) {
        KarbeatLogger.error("Error executing custom backend change: $e");
      }
    });
  }

  // ============== GETTERS =================
  TransportState get transport => _transportState;
  ProjectMetadata get metadata => _metadata;
  bool get isPlaying => _transportState.isPlaying;
  bool get isPatternPlaying => _transportState.isPatternPlaying;
  bool get isSongPlaying =>
      _transportState.isPlaying && !_transportState.isPatternPlaying;
  bool get isLooping => _transportState.isLooping;
  double get tempo => _transportState.bpm;
  Map<int, UiTrack> get tracks => _tracks;
  Map<int, AudioWaveformUiForAudioProperties> get audioSources => _audioSources;
  Map<int, UiGeneratorInstance> get generators => _generators;
  ToolSelection get selectedTool => _selectedTool;
  WorkspaceView get currentView => _currentView;
  ToolbarMenuContextGroup get currentToolbarContext => _currentToolbarContext;
  AudioHardwareConfig get hardwareConfig => _hardwareConfig;
  Stream<PlaybackPosition> get positionStream => _positionBroadcastStream;
  Map<int, UiPattern> get patterns => _patterns;
  int get pianoRollGridDenom => _piannoRollGridDenom;
  int? get editingPatternId => _editingPatternId;
  InteractionTarget? get interactionTarget => _interactionTarget;
  mixer_api.UiMixerState get mixerState => _mixerState;

  // Session state getters (frontend-only)
  int? get selectedTrackId => _selectedTrackId;
  List<int> get selectedClipIds => _selectedClipIds;
  int? get focusClipId => _focusClipId;

  // Piano roll getters
  PianoRollToolSelection get pianoRollTool => _pianoRollTool;
  Set<int> get selectedNoteIds => _selectedNoteIds;
  int? get previewGeneratorId => _previewGeneratorId;

  // ================ SETTERS ===================
  set pianoRollGridDenom(GridValue val) {
    _piannoRollGridDenom = val.value;
  }

  // =============== GLOBAL UI STATE ==========================
  double _horizontalZoomLevel = 1000;
  double get horizontalZoomLevel => _horizontalZoomLevel;

  /// Min: 1 sample/px (each sample tick visible). Max: 100k samples/px.
  static const double _minZoom = 1.0;
  static const double _maxZoom = 100000.0;

  set horizontalZoomLevel(double val) {
    final clamped = val.clamp(_minZoom, _maxZoom);
    if (_horizontalZoomLevel != clamped) {
      _horizontalZoomLevel = clamped;
      notifyListeners();
    }
  }

  Map<int, int> trackIdHeightMap = {};

  // =============== PLACEMENT MODE STATE (USED WHEN AUDIO CLIP PLACEMENT) =====================
  int? _placingSourceId;
  UiSourceType? _placingSourceType;
  int? get placingSourceId => _placingSourceId;

  // Track where the user wants to drop it
  double _placementTimeSamples = 0.0;
  int _placementTrackId = -1;

  bool get isPlacing => _placingSourceId != null;

  // ================ SYNCHRONIZATION ======================

  /// Syncs only the track state
  Future<void> syncTrackState() async {
    try {
      final newState = await getTracks();
      _tracks = newState;
      notifyListeners();
    } catch (e) {
      KarbeatLogger.error("error when syncing the track state: $e");
    }
  }

  /// Syncs only the transport (Playhead, Play state)
  /// Call this inside a Ticker (e.g. 60Hz)
  Future<void> syncTransportState() async {
    try {
      final newState = await getTransportState();
      // Optimization: Only notify if changed significantly (optional)
      _transportState = newState;
      notifyListeners();
    } catch (e) {
      KarbeatLogger.error("Transport sync failed: $e");
    }
  }

  /// Syncs only the metadata (Project name, BPM, Time signature)
  Future<void> syncMetadataState() async {
    try {
      final newState = await getProjectMetadata();
      _metadata = newState;
      notifyListeners();
    } catch (e) {
      KarbeatLogger.error("failed when syncing metadata: $e");
    }
  }

  Future<void> syncMaxSampleIndex() async {
    try {
      final newState = await getMaxSampleIndex();
      maxSamplesIndex = newState;
      notifyListeners();
    } catch (e) {
      KarbeatLogger.error("failed when syncing max sample index: $e");
    }
  }

  /// Syncs the list of loaded audio files
  /// Call this when: Adding a file, Removing a file
  Future<void> syncAudioSourceList() async {
    final sources = await getAudioSourceList();
    if (sources != null) {
      _audioSources = Map.from(sources);
      notifyListeners();
    }
  }

  Future<void> syncGeneratorList() async {
    try {
      final generators = await getGeneratorList();
      _generators = generators;
      notifyListeners();
    } catch (e) {
      KarbeatLogger.error("Failed to sync generators: $e");
    }
  }

  Future<void> syncGenerator({required int generatorId}) async {
    try {
      final generator = await getGenerator(generatorId: generatorId);
      final newMap = Map<int, UiGeneratorInstance>.from(_generators);
      newMap[generatorId] = generator;
      _generators = newMap;
      notifyListeners();
    } catch (error) {
      KarbeatLogger.error("Failed to sync generator $generatorId: $error");
    }
  }

  Future<void> syncAudioHardwareConfigState() async {
    try {
      final newState = await getAudioConfig();
      _hardwareConfig = newState;
      notifyListeners();
    } catch (e) {
      KarbeatLogger.error("Failed to sync audio hardware state: $e");
    }
  }

  Future<void> syncPatternList() async {
    try {
      final result = await getPatterns();
      _patterns = result;
      notifyListeners();
    } catch (e) {
      KarbeatLogger.error("Failed to sync pattern list: $e");
    }
  }

  /// Efficiently syncs a SINGLE pattern instead of the whole list
  Future<void> syncPattern(int patternId) async {
    try {
      final updatedPattern = await getPattern(patternId: patternId);

      // Creating a new map reference ensures Selectors in UI will trigger a rebuild
      final newMap = Map<int, UiPattern>.from(_patterns);
      newMap[patternId] = updatedPattern;
      _patterns = newMap;

      notifyListeners();
    } catch (e) {
      KarbeatLogger.error("Error syncing single pattern $patternId: $e");
    }
  }

  /// Efficiently syncs a SINGLE track (and its clips)
  Future<void> syncTrack(int trackId) async {
    try {
      final updatedTrack = await getTrack(trackId: trackId);
      final newMap = Map<int, UiTrack>.from(_tracks);
      newMap[trackId] = updatedTrack;
      _tracks = newMap;

      notifyListeners();
    } catch (e) {
      KarbeatLogger.error("Error syncing single track $trackId: $e");
    }
  }

  /// Triggers a state refresh. Call this after any Rust API action.
  void notifyBackendChange(ProjectEvent event) {
    if (!_stateEventController.isClosed) {
      _stateEventController.add(event);
    }
  }

  /// Triggers a state refresh. Call this after any Rust API action.
  void notifyCustomBackendChange(Future<void> Function() action) {
    if (!_customStateEventController.isClosed) {
      _customStateEventController.add(action);
    }
  }

  Future<void> syncMixerState() async {
    try {
      final newState = await mixer_api.getMixerState();
      _mixerState = newState;
      notifyListeners();
    } catch (e) {
      KarbeatLogger.error("Failed to sync mixer state: $e");
    }
  }

  Future<void> syncBuses() async {
    try {
      final newBuses = await mixer_api.getBuses();
      _mixerState = _mixerState.copyWith(buses: newBuses);
      notifyListeners();
    } catch (e) {
      KarbeatLogger.error("Failedto sync mixer bus: $e");
    }
  }

  Future<void> syncMixerChannel(int trackId) async {
    try {
      final updatedChannel = await mixer_api.getMixerChannel(trackId: trackId);
      final newChannels = Map<int, mixer_api.UiMixerChannel>.from(
        _mixerState.channels,
      );
      newChannels[trackId] = updatedChannel;
      _mixerState = _mixerState.copyWith(channels: newChannels);
      notifyListeners();
    } catch (e) {
      KarbeatLogger.error("Error syncing mixer channel $trackId: $e");
    }
  }

  Future<void> syncMasterBus() async {
    try {
      final updatedMaster = await mixer_api.getMasterBus();
      _mixerState = _mixerState.copyWith(masterBus: updatedMaster);
      notifyListeners();
    } catch (e) {
      KarbeatLogger.error("Failed to sync master bus: $e");
    }
  }

  // =============== ACTIONS ===============

  /// Loads an audio file and refreshes the source list
  Future<Result<void>> addAudioFile(String path) async {
    try {
      await addAudioSource(filePath: path);
      notifyBackendChange(ProjectEvent.sourceListChanged);
      return Result.ok(null);
    } catch (e) {
      KarbeatLogger.error("Failed to add audio file: $e");
      return Result.error(Exception("$e"));
    }
  }

  Future<Result<void>> addTrack(TrackType type) async {
    try {
      await addNewTrack(trackType: type);
      notifyBackendChange(ProjectEvent.tracksChanged);
      return Result.ok(null);
    } catch (e) {
      KarbeatLogger.error("Failed to add track: $e");
      return Result.error(Exception("$e"));
    }
  }

  /// Add a MIDI track with a generator by its registry ID (preferred method).
  Future<Result<void>> addMidiTrackWithGeneratorId(int registryId) async {
    try {
      await track_api.addMidiTrackWithGeneratorId(registryId: registryId);
      notifyBackendChange(ProjectEvent.tracksChanged);
      notifyBackendChange(ProjectEvent.generatorListChanged);
      return Result.ok(null);
    } catch (e) {
      KarbeatLogger.error("Failed to add midi track: $e");
      return Result.error(Exception("$e"));
    }
  }

  /// Add a MIDI track with a generator by name (backwards compatible).
  Future<Result<void>> addMidiTrackWithGenerator(String generatorName) async {
    try {
      await track_api.addMidiTrackWithGenerator(generatorName: generatorName);
      notifyCustomBackendChange(() async {
        await syncTrackState();
        await syncGeneratorList();
      });
      return Result.ok(null);
    } catch (e) {
      KarbeatLogger.error("Failed to add midi track: $e");
      return Result.error(Exception("$e"));
    }
  }

  Future<Result<void>> addEffectToMixerChannel(
    int channelId,
    int registryId,
  ) async {
    try {
      if (channelId == -1) {
        KarbeatLogger.info("Adding effect to master channel");
        await mixer_api.addEffectToMasterBus(registryId: registryId);
        notifyCustomBackendChange(() async {
          await syncMasterBus();
        });
      } else {
        KarbeatLogger.info("Adding effect to track channel $channelId");
        await mixer_api.addEffectToMixerChannelById(
          trackId: channelId,
          registryId: registryId,
        );
        notifyCustomBackendChange(() async {
          await syncMixerChannel(channelId);
        });
      }
      return Result.ok(null);
    } catch (e) {
      KarbeatLogger.error("Failed to add effect to channel: $e");
      return Result.error(Exception("$e"));
    }
  }

  Future<Result<void>> addEffectToMasterBus(int registryId) async {
    try {
      await mixer_api.addEffectToMasterBus(registryId: registryId);
      notifyCustomBackendChange(() async {
        await syncMasterBus();
        KarbeatLogger.info("Adding effect to master channel");
      });
      return Result.ok(null);
    } catch (e) {
      KarbeatLogger.error("Failed to add effect to master bus: $e");
      return Result.error(Exception("$e"));
    }
  }

  Future<Result<void>> togglePlay() async {
    try {
      // If pattern is playing, the user's intent is to switch to song playback.
      // Since pattern mode sets isPlaying=true, !isPlaying would be false (stop),
      // so we force newPlaying=true to start song mode instead.
      KarbeatLogger.info(
        "Toggle play, is playing: ${_transportState.isPlaying}, is pattern playing: ${_transportState.isPatternPlaying}",
      );
      final newPlaying = _transportState.isPatternPlaying
          ? true
          : !_transportState.isPlaying;

      if (newPlaying) {
        _pendingPlayRequest = true;
      }

      await transport_api.setPlaying(val: newPlaying);
      notifyBackendChange(ProjectEvent.transportChanged);
      return Result.ok(null);
    } catch (e) {
      log("Failed to toggle play: $e");
      // Reset flag on error
      _pendingPlayRequest = false;
      return Result.error(Exception("$e"));
    }
  }

  Future<Result<void>> stop() async {
    try {
      await transport_api.stopSongPlayback();
      notifyBackendChange(ProjectEvent.transportChanged);
      return Result.ok(null);
    } catch (e) {
      KarbeatLogger.error("Failed to stop play: $e");
      return Result.error(Exception("$e"));
    }
  }

  Future<Result<void>> toggleLoop() async {
    try {
      final newLooping = !_transportState.isLooping;
      await transport_api.setLooping(val: newLooping);
      await syncTransportState();
      return Result.ok(null);
    } catch (e) {
      KarbeatLogger.error("Failed to toggle loop: $e");
      return Result.error(Exception("$e"));
    }
  }

  /// Sets the BPM.
  /// Updates local state optimistically and calls the backend API.
  /// Auto-scales horizontalZoomLevel so that grid lines remain visually fixed.
  Future<Result<void>> setBpm(double value) async {
    try {
      final oldBpm = _transportState.bpm;
      _transportState = _transportState.copyWith(bpm: value);

      // Scale zoom so that (samplesPerBeat / zoomLevel) stays constant,
      // keeping grid lines at the same pixel positions.
      if (oldBpm > 0 && value > 0) {
        horizontalZoomLevel = _horizontalZoomLevel * (oldBpm / value);
      }

      notifyListeners();

      await transport_api.setBpm(val: value);
      notifyBackendChange(ProjectEvent.transportChanged);
      return Result.ok(null);
    } catch (e) {
      KarbeatLogger.error("Failed to set bpm: $e");
      return Result.error(Exception("$e"));
    }
  }

  /// Change the selected tool to a desired tool
  void selectTool(ToolSelection tool) {
    if (_selectedTool != tool) {
      _selectedTool = tool;
      notifyListeners();
    }
  }

  // =============== PIANO ROLL ACTIONS ===============

  /// Change the selected piano roll tool
  void selectPianoRollTool(PianoRollToolSelection tool) {
    if (_pianoRollTool != tool) {
      _pianoRollTool = tool;
      notifyListeners();
    }
  }

  /// Select notes in the piano roll
  void selectNotes(Set<int> noteIds) {
    _selectedNoteIds = noteIds;
    notifyListeners();
  }

  /// Add notes to the current selection
  void addNotesToSelection(Set<int> noteIds) {
    _selectedNoteIds = {..._selectedNoteIds, ...noteIds};
    notifyListeners();
  }

  /// Clear note selection
  void clearNoteSelection() {
    if (_selectedNoteIds.isNotEmpty) {
      _selectedNoteIds = {};
      notifyListeners();
    }
  }

  void toggleToolbarContext(ToolbarMenuContextGroup group) {
    if (group == _currentToolbarContext) {
      // Toggle off
      _currentToolbarContext = ToolbarMenuContextGroup.none;
    } else {
      _currentToolbarContext = group;
    }
    notifyListeners();
  }

  void closeContextPanel() {
    _currentToolbarContext = ToolbarMenuContextGroup.none;
    notifyListeners();
  }

  /// Shows the interaction panel for a given target (clip, multi-clip, or track)
  void showInteractionPanel(InteractionTarget target) {
    _interactionTarget = target;
    notifyListeners();
  }

  /// Hides the interaction panel
  void hideInteractionPanel() {
    if (_interactionTarget != null) {
      _interactionTarget = null;
      notifyListeners();
    }
  }

  void navigateTo(WorkspaceView view) {
    if (_currentView != view) {
      _currentView = view;
      notifyListeners();
    }
  }

  /// Opens the piano roll view with a specific pattern (from source list).
  void openPattern(int patternId) {
    _editingPatternId = patternId;
    navigateTo(WorkspaceView.pianoRoll);
  }

  void setGridSize(int newSize) {
    if (gridSize != newSize) {
      gridSize = newSize;
      notifyListeners();
    }
  }

  void setHorizontalZoom(double level) {
    if (horizontalZoomLevel != level) {
      horizontalZoomLevel = level;
      notifyListeners();
    }
  }

  Future<Result<void>> seekTo(int samples) async {
    try {
      // Call the Rust API
      await transport_api.setPlayhead(val: samples);

      // Optimistic update (optional, since Rust pushes the update back immediately)
      notifyListeners();
      return Result.ok(null);
    } catch (e) {
      KarbeatLogger.error("Error seeking: $e");
      return Result.error(Exception("$e"));
    }
  }

  Future<Result<void>> deleteClip(int trackId, int clipId) async {
    if (_tracks.containsKey(trackId)) {
      final track = _tracks[trackId]!;
      final updatedClips = track.clips.where((c) => c.id != clipId).toList();

      _tracks = Map.from(_tracks);
      _tracks[trackId] = _copyWithTrack(track, clips: updatedClips);
      notifyListeners();
    }

    try {
      await track_api.deleteClip(trackId: trackId, clipId: clipId);
      // notifyBackendChange(ProjectEvent.tracksChanged);
      await syncTrack(trackId);
      return Result.ok(null);
    } catch (e) {
      KarbeatLogger.error("Error deleting clip: $e");
      // await syncTrackState();
      return Result.error(Exception("$e"));
    }
  }

  Future<Result<void>> resizeClip(
    int trackId,
    int clipId,
    ResizeEdge edge,
    int newTime,
  ) async {
    _applyOptimisticResize(trackId, clipId, edge, newTime);
    try {
      await track_api.resizeClip(
        trackId: trackId,
        clipId: clipId,
        edge: edge,
        newTimeVal: newTime,
      );
      await syncTrack(trackId);
      return Result.ok(null);
    } catch (e) {
      KarbeatLogger.error("Error resizing clip: $e");
      // await syncTrackState();
      return Result.error(Exception("$e"));
    }
  }

  Future<Result<void>> moveClip(
    int trackId,
    int clipId,
    int newStartTime, {
    int? newTrackId,
  }) async {
    try {
      await track_api.moveClip(
        sourceTrackId: trackId,
        clipId: clipId,
        newStartTime: newStartTime,
        newTrackId: newTrackId,
      );
      // notifyBackendChange(ProjectEvent.tracksChanged);

      // Fetch the old track ID and new track ID
      await syncTrack(trackId);
      if (newTrackId != null && newTrackId != trackId) {
        await syncTrack(newTrackId);
      }
      return Result.ok(null);
    } catch (e) {
      KarbeatLogger.error("Error moving clip: $e");
      // await syncTrackState();
      return Result.error(Exception("$e"));
    }
  }

  Future<Result<void>> createEmptyPatternClip({
    required int trackId,
    required int startTime,
  }) async {
    try {
      await createClip(
        sourceType: UiSourceType.midi,
        trackId: trackId,
        startTime: startTime,
      );
      KarbeatLogger.info("New empty pattern clip is successfully created");
      // notifyBackendChange(ProjectEvent.tracksChanged);
      await syncTrack(trackId);
      return Result.ok(null);
    } catch (e) {
      KarbeatLogger.error("Error when creating new empty pattern clip: $e");
      return Result.error(Exception("$e"));
    }
  }

  // ===================== BATCH CLIP OPERATIONS ==========================

  /// Move multiple clips by a delta amount (in samples)
  Future<Result<void>> moveClipBatch(
    int trackId,
    List<int> clipIds,
    int deltaSamples, {
    int? newTrackId,
  }) async {
    try {
      await track_api.moveClipBatch(
        sourceTrackId: trackId,
        clipIds: clipIds,
        deltaSamples: deltaSamples,
        newTrackId: newTrackId,
      );
      await syncTrack(trackId);
      if (newTrackId != null && newTrackId != trackId) {
        await syncTrack(newTrackId);
      }
      return Result.ok(null);
    } catch (e) {
      KarbeatLogger.error("Error moving clips in batch: $e");
      return Result.error(Exception("$e"));
    }
  }

  /// Resize multiple clips by a delta amount (in samples)
  Future<Result<void>> resizeClipBatch(
    int trackId,
    List<int> clipIds,
    ResizeEdge edge,
    int deltaSamples,
  ) async {
    try {
      await track_api.resizeClipBatch(
        trackId: trackId,
        clipIds: clipIds,
        edge: edge,
        deltaSamples: deltaSamples,
      );
      await syncTrack(trackId);
      return Result.ok(null);
    } catch (e) {
      KarbeatLogger.error("Error resizing clips in batch: $e");
      return Result.error(Exception("$e"));
    }
  }

  /// Delete multiple clips at once
  Future<Result<void>> deleteClipBatch(int trackId, List<int> clipIds) async {
    // Optimistic update
    if (_tracks.containsKey(trackId)) {
      final track = _tracks[trackId]!;
      final clipIdSet = clipIds.toSet();
      final updatedClips = track.clips
          .where((c) => !clipIdSet.contains(c.id))
          .toList();

      _tracks = Map.from(_tracks);
      _tracks[trackId] = _copyWithTrack(track, clips: updatedClips);
      notifyListeners();
    }

    try {
      await track_api.deleteClipBatch(trackId: trackId, clipIds: clipIds);
      await syncTrack(trackId);
      return Result.ok(null);
    } catch (e) {
      KarbeatLogger.error("Error deleting clips in batch: $e");
      return Result.error(Exception("$e"));
    }
  }

  /// Convenience method to delete all currently selected clips
  Future<Result<void>> deleteSelectedClips() async {
    final trackId = _selectedTrackId;
    final clipIds = _selectedClipIds;

    if (trackId == null || clipIds.isEmpty) return Result.ok(null);

    final result = await deleteClipBatch(trackId, clipIds);
    deselectAllClips();
    return result;
  }

  // ===================== NOTE CHANGE API'S ==========================
  Future<Result<void>> previewNote({
    required int trackId,
    required int noteKey,
    required bool isOn,
    int velocity = 0,
  }) async {
    try {
      await playPreviewNote(
        trackId: trackId,
        noteKey: noteKey,
        velocity: velocity,
        isOn: isOn,
      );
      KarbeatLogger.info(
        "Play ${numToMidiKey(noteKey)} with generator from $trackId",
      );
      return Result.ok(null);
    } catch (e) {
      KarbeatLogger.error("Error previewing note: $e");
      return Result.error(Exception("$e"));
    }
  }

  Future<Result<void>> addPatternNote({
    required int patternId,
    required int key,
    required int startTick,
    required int duration,
  }) async {
    try {
      await addNote(
        patternId: patternId,
        key: key,
        startTick: startTick,
        duration: duration,
      );
      // notifyBackendChange(ProjectEvent.patternChanged);
      await syncPattern(patternId);
      return Result.ok(null);
    } catch (e) {
      KarbeatLogger.error("Error adding note: $e");
      return Result.error(Exception("$e"));
    }
  }

  Future<Result<void>> deletePatternNote({
    required int patternId,
    required int noteId,
  }) async {
    _applyOptimisticNoteDeletion(patternId, noteId);
    try {
      await deleteNote(patternId: patternId, noteId: noteId);
      // notifyBackendChange(ProjectEvent.patternChanged);
      await syncPattern(patternId);
      return Result.ok(null);
    } catch (e) {
      KarbeatLogger.error("Error deleting note: $e");
      await syncPattern(patternId);
      return Result.error(Exception("$e"));
    }
  }

  Future<Result<void>> movePatternNote({
    required int patternId,
    required int noteId,
    required int newStartTick,
    required int newKey,
  }) async {
    try {
      await moveNote(
        patternId: patternId,
        noteId: noteId,
        newStartTick: newStartTick,
        newKey: newKey,
      );
      // notifyBackendChange(ProjectEvent.patternChanged);
      await syncPattern(patternId);
      return Result.ok(null);
    } catch (e) {
      KarbeatLogger.error("Error moving note: $e");
      return Result.error(Exception("$e"));
    }
  }

  Future<Result<void>> resizePatternNote({
    required int patternId,
    required int noteId,
    required int newDuration,
  }) async {
    try {
      await resizeNote(
        patternId: patternId,
        noteId: noteId,
        newDuration: newDuration,
      );
      // notifyBackendChange(ProjectEvent.patternChanged);
      await syncPattern(patternId);
      return Result.ok(null);
    } catch (e) {
      KarbeatLogger.error("Error resizing note: $e");
      return Result.error(Exception("$e"));
    }
  }

  // ==================== OPTIMISTIC HELPERS =============================
  // Helper
  void _applyOptimisticResize(
    int trackId,
    int clipId,
    ResizeEdge edge,
    int newTime,
  ) {
    final track = _tracks[trackId];
    if (track == null) return;

    final clipIndex = track.clips.indexWhere((c) => c.id == clipId);
    if (clipIndex == -1) return;

    final clip = track.clips[clipIndex];

    int newStart = clip.startTime.toInt();
    int newLength = clip.loopLength.toInt();
    int newOffset = clip.offsetStart.toInt();

    if (edge == ResizeEdge.right) {
      // Dragging Right Edge: newTime is the END time
      if (newTime > clip.startTime) {
        newLength = newTime - clip.startTime;
      }
    } else {
      // Dragging Left Edge: newTime is the START time (Slip Edit)
      final oldEnd = clip.startTime + clip.loopLength;

      if (newTime < oldEnd) {
        final delta = newTime - clip.startTime;
        final potentialOffset = clip.offsetStart + delta;

        // Constraint: Offset cannot be negative
        if (potentialOffset >= 0) {
          newStart = newTime;
          newLength = oldEnd - newTime;
          newOffset = potentialOffset.toInt();
        }
      }
    }

    // Create Updated Objects
    final updatedClip = UiClip(
      id: clip.id,
      name: clip.name,
      startTime: newStart,
      loopLength: newLength,
      offsetStart: newOffset,
      source: clip.source,
    );

    final updatedClips = List<UiClip>.from(track.clips);
    updatedClips[clipIndex] = updatedClip;

    final updatedTrack = _copyWithTrack(track, clips: updatedClips);

    _tracks = Map.from(_tracks);
    _tracks[trackId] = updatedTrack;

    notifyListeners();
  }

  void _applyOptimisticNoteDeletion(int patternId, int noteId) {
    final pattern = _patterns[patternId];
    if (pattern == null) return;

    // Filter out the specific note
    final updatedNotes = pattern.notes.where((n) => n.id != noteId).toList();

    final updatedPattern = UiPattern(
      id: pattern.id,
      name: pattern.name,
      lengthTicks: pattern.lengthTicks,
      notes: updatedNotes,
    );

    // Update Store & Notify UI immediately
    final newPatterns = Map<int, UiPattern>.from(_patterns);
    newPatterns[patternId] = updatedPattern;
    _patterns = newPatterns;

    notifyListeners();
  }

  // ============= PLACEMENT MODE LOGIC =================

  void startPlacement(int sourceId, {required UiSourceType type}) {
    _placingSourceId = sourceId;
    _placingSourceType = type;
    // Switch view to track list immediately so user can place it
    navigateTo(WorkspaceView.trackList);
    notifyListeners();
  }

  /// Updates the target location without notifying all listeners
  /// (Use setState in the UI for visual feedback to avoid global rebuilds)
  void updatePlacementTarget(int trackId, double timeSamples) {
    _placementTrackId = trackId;
    _placementTimeSamples = timeSamples;
  }

  Future<Result<void>> confirmPlacement() async {
    KarbeatLogger.info("CONFIRM Placement");
    if (_placingSourceId != null &&
        _placingSourceType != null &&
        _placementTrackId != -1) {
      try {
        await createClip(
          sourceId: _placingSourceId!,
          sourceType: _placingSourceType!,
          trackId: _placementTrackId,
          startTime: _placementTimeSamples.toInt(),
        );

        // Reset mode
        _placingSourceId = null;
        _placingSourceType = null;
        _placementTrackId = -1;

        notifyBackendChange(ProjectEvent.tracksChanged);
        return Result.ok(null);
      } catch (e) {
        KarbeatLogger.error("Error creating clip: $e");
        return Result.error(Exception("$e"));
      }
    }
    return Result.ok(null);
  }

  void cancelPlacement() {
    _placingSourceId = null;
    _placingSourceType = null;
    _placementTrackId = -1;
    notifyListeners();
  }

  UiTrack _copyWithTrack(UiTrack original, {List<UiClip>? clips}) {
    return UiTrack(
      id: original.id,
      color: "#FFFFFF",
      name: original.name,
      trackType: original.trackType,
      clips: clips ?? original.clips,
    );
  }

  // ================== Session State (frontend-only) =====================

  /// Select a single clip (replaces any existing selection)
  void selectClip({required int trackId, required int clipId}) {
    _selectedTrackId = trackId;
    _selectedClipIds = [clipId];
    _focusClipId = clipId;
    notifyListeners();
  }

  /// Add a clip to the current selection (for Ctrl+Click)
  void addClipToSelection({required int trackId, required int clipId}) {
    // Different track - clear and start fresh
    if (_selectedTrackId != null && _selectedTrackId != trackId) {
      _selectedClipIds = [];
    }
    _selectedTrackId = trackId;
    if (!_selectedClipIds.contains(clipId)) {
      _selectedClipIds = [..._selectedClipIds, clipId];
    }
    _focusClipId = clipId;
    notifyListeners();
  }

  /// Remove a clip from the current selection
  void removeClipFromSelection({required int clipId}) {
    _selectedClipIds = _selectedClipIds.where((id) => id != clipId).toList();

    // If we removed the focus clip, update focus to last selected
    if (_focusClipId == clipId) {
      _focusClipId = _selectedClipIds.isNotEmpty ? _selectedClipIds.last : null;
    }

    // If no clips left, clear the track selection too
    if (_selectedClipIds.isEmpty) {
      _selectedTrackId = null;
      _focusClipId = null;
    }
    notifyListeners();
  }

  /// Select multiple clips at once (for range select)
  void selectClips({required int trackId, required List<int> clipIds}) {
    _selectedTrackId = trackId;
    _selectedClipIds = List.from(clipIds);
    _focusClipId = clipIds.isNotEmpty ? clipIds.last : null;
    notifyListeners();
  }

  /// Clear all clip selection
  void deselectAllClips() {
    _selectedTrackId = null;
    _selectedClipIds = [];
    _focusClipId = null;
    notifyListeners();
  }

  /// Set the preview generator for piano roll
  void setPreviewGenerator({int? generatorId}) {
    _previewGeneratorId = generatorId;
    notifyListeners();
  }

  // ====================================================
  // ================== Mixer API's =====================
  // ====================================================

  // ================ MIXER EVENT STREAM ==================

  /// Subscribe to the Rust → Dart mixer param event stream.
  void _initMixerEventStream() {
    _mixerEventSubscription?.cancel();
    _mixerEventSubscription = mixer_api.createMixerEventStream().listen(
      (event) {
        _applyMixerParamLocally(event);
      },
      onError: (e) {
        KarbeatLogger.error('Mixer event stream error: $e');
      },
    );
  }

  /// Apply a single mixer param event to local state, skipping touched params.
  void _applyMixerParamLocally(mixer_api.MixerParamEvent event) {
    final int trackId = event.trackId;
    final bool isMaster = (trackId == 4294967295); // u32::MAX

    mixer_api.UiMixerChannel channel;
    if (isMaster) {
      channel = _mixerState.masterBus;
    } else {
      final existing = _mixerState.channels[trackId];
      if (existing == null) return;
      channel = existing;
    }

    double volume = channel.volume;
    double pan = channel.pan;
    bool mute = channel.mute;
    bool solo = channel.solo;
    bool changed = false;

    if (event.volume != null && !_touchedParams.contains((trackId, 'volume'))) {
      volume = event.volume!;
      changed = true;
    }
    if (event.pan != null && !_touchedParams.contains((trackId, 'pan'))) {
      pan = event.pan!;
      changed = true;
    }
    if (event.mute != null && !_touchedParams.contains((trackId, 'mute'))) {
      mute = event.mute!;
      changed = true;
    }
    if (event.solo != null && !_touchedParams.contains((trackId, 'solo'))) {
      solo = event.solo!;
      changed = true;
    }

    if (!changed) return;

    final updatedChannel = mixer_api.UiMixerChannel(
      volume: volume,
      pan: pan,
      mute: mute,
      solo: solo,
      invertedPhase: channel.invertedPhase,
      effects: channel.effects,
    );

    if (isMaster) {
      _mixerState = mixer_api.UiMixerState.newWithParam(
        channels: _mixerState.channels,
        masterBus: updatedChannel,
        buses: _mixerState.buses,
        routing: _mixerState.routing,
      );
    } else {
      final newChannels = Map<int, mixer_api.UiMixerChannel>.from(
        _mixerState.channels,
      );
      newChannels[trackId] = updatedChannel;
      _mixerState = mixer_api.UiMixerState.newWithParam(
        channels: newChannels,
        masterBus: _mixerState.masterBus,
        buses: _mixerState.buses,
        routing: _mixerState.routing,
      );
    }

    notifyListeners();
  }

  /// Mark a mixer param as "touched" (user is actively dragging).
  /// Automation events for this param will be ignored while touched.
  void markParamTouched(int trackId, String paramName) {
    _touchedParams.add((trackId, paramName));
  }

  /// Mark a mixer param as "released" (user finished dragging).
  /// Automation events will resume for this param.
  void markParamReleased(int trackId, String paramName) {
    _touchedParams.remove((trackId, paramName));
  }

  Future<Result<void>> setMixerChannelParams({
    required int trackId,
    required List<mixer_api.UiMixerChannelParams> params,
  }) async {
    // Optimistic local update so the controlled Slider doesn't snap back
    _applyParamsToLocalChannel(trackId, params, isMaster: false);

    try {
      await mixer_api.setMixerChannelParams(trackId: trackId, params: params);
      // No need for notifyBackendChange here — the optimistic update already
      // notified listeners, and the event stream will keep us in sync.
      return Result.ok(null);
    } catch (e) {
      KarbeatLogger.error('Error setting mixer channel params: $e');
      // On error, re-sync from the backend to undo the optimistic update
      syncMixerState();
      return Result.error(Exception("$e"));
    }
  }

  Future<Result<void>> setMasterBusParams({
    required List<mixer_api.UiMixerChannelParams> params,
  }) async {
    // Optimistic local update so the controlled Slider doesn't snap back
    _applyParamsToLocalChannel(0, params, isMaster: true);

    try {
      await mixer_api.setMasterBusParams(params: params);
      return Result.ok(null);
    } catch (e) {
      KarbeatLogger.error('Error setting master bus params: $e');
      syncMixerState();
      return Result.error(Exception("$e"));
    }
  }

  Future<Result<void>> setBusChannelParams({
    required int busId,
    required List<mixer_api.UiMixerChannelParams> params,
  }) async {
    // Optimistic local update so the controlled Slider doesn't snap back
    _applyParamsToBusChannel(busId, params);

    try {
      await mixer_api.setBusParams(busId: busId, params: params);
      return Result.ok(null);
    } catch (e) {
      KarbeatLogger.error('Error setting bus channel params: $e');
      syncMixerState();
      return Result.error(Exception("$e"));
    }
  }

  Future<Result<void>> createNewBusChannel({String name = "Untitled"}) async {
    try {
      await mixer_api.createBus(name: name);
      notifyCustomBackendChange(() async {
        await syncBuses();
      });
      return Result.ok(null);
    } catch (e) {
      KarbeatLogger.error('Error when trying to add a new bus channel: $e');
      syncBuses();
      return Result.error(Exception("$e"));
    }
  }

  /// Immediately apply param changes to local _mixerState and notify listeners.
  void _applyParamsToLocalChannel(
    int trackId,
    List<mixer_api.UiMixerChannelParams> params, {
    required bool isMaster,
  }) {
    final channel = isMaster
        ? _mixerState.masterBus
        : _mixerState.channels[trackId];
    if (channel == null) return;

    double volume = channel.volume;
    double pan = channel.pan;
    bool mute = channel.mute;
    bool solo = channel.solo;
    bool invertedPhase = channel.invertedPhase;

    for (final p in params) {
      switch (p) {
        case mixer_api.UiMixerChannelParams_Volume():
          volume = p.field0;
        case mixer_api.UiMixerChannelParams_Pan():
          pan = p.field0;
        case mixer_api.UiMixerChannelParams_Mute():
          mute = p.field0;
        case mixer_api.UiMixerChannelParams_Solo():
          solo = p.field0;
        case mixer_api.UiMixerChannelParams_InvertedPhase():
          invertedPhase = p.field0;
      }
    }

    final updated = mixer_api.UiMixerChannel(
      volume: volume,
      pan: pan,
      mute: mute,
      solo: solo,
      invertedPhase: invertedPhase,
      effects: channel.effects,
    );

    if (isMaster) {
      _mixerState = mixer_api.UiMixerState.newWithParam(
        channels: _mixerState.channels,
        masterBus: updated,
        buses: _mixerState.buses,
        routing: _mixerState.routing,
      );
    } else {
      final newChannels = Map<int, mixer_api.UiMixerChannel>.from(
        _mixerState.channels,
      );
      newChannels[trackId] = updated;
      _mixerState = mixer_api.UiMixerState.newWithParam(
        channels: newChannels,
        masterBus: _mixerState.masterBus,
        buses: _mixerState.buses,
        routing: _mixerState.routing,
      );
    }

    notifyListeners();
  }

  /// Immediately apply param changes to a bus in local _mixerState and notify listeners.
  void _applyParamsToBusChannel(
    int busId,
    List<mixer_api.UiMixerChannelParams> params,
  ) {
    final bus = _mixerState.buses[busId];
    if (bus == null) return;

    final channel = bus.channel;
    double volume = channel.volume;
    double pan = channel.pan;
    bool mute = channel.mute;
    bool solo = channel.solo;
    bool invertedPhase = channel.invertedPhase;

    for (final p in params) {
      switch (p) {
        case mixer_api.UiMixerChannelParams_Volume():
          volume = p.field0;
        case mixer_api.UiMixerChannelParams_Pan():
          pan = p.field0;
        case mixer_api.UiMixerChannelParams_Mute():
          mute = p.field0;
        case mixer_api.UiMixerChannelParams_Solo():
          solo = p.field0;
        case mixer_api.UiMixerChannelParams_InvertedPhase():
          invertedPhase = p.field0;
      }
    }

    final updatedChannel = mixer_api.UiMixerChannel(
      volume: volume,
      pan: pan,
      mute: mute,
      solo: solo,
      invertedPhase: invertedPhase,
      effects: channel.effects,
    );

    final updatedBus = mixer_api.UiBus(
      id: bus.id,
      name: bus.name,
      channel: updatedChannel,
    );

    final newBuses = Map<int, mixer_api.UiBus>.from(_mixerState.buses);
    newBuses[busId] = updatedBus;
    _mixerState = mixer_api.UiMixerState.newWithParam(
      channels: _mixerState.channels,
      masterBus: _mixerState.masterBus,
      buses: newBuses,
      routing: _mixerState.routing,
    );

    notifyListeners();
  }
}

extension on mixer_api.UiMixerState {
  mixer_api.UiMixerState copyWith({
    Map<int, mixer_api.UiMixerChannel>? channels,
    mixer_api.UiMixerChannel? masterBus,
    Map<int, mixer_api.UiBus>? buses,
    List<mixer_api.UiRoutingConnection>? routing,
  }) {
    return mixer_api.UiMixerState.newWithParam(
      channels: channels ?? this.channels,
      masterBus: masterBus ?? this.masterBus,
      buses: buses ?? this.buses,
      routing: routing ?? this.routing,
    );
  }
}

extension TransportStateCopyWith on TransportState {
  TransportState copyWith({
    bool? isPlaying,
    bool? isPatternPlaying,
    bool? isRecording,
    bool? isLooping,
    int? playheadPositionSamples,
    int? loopStartSamples,
    int? loopEndSamples,
    double? bpm,
    (int, int)? timeSignature,
    int? barTracker,
    int? beatTracker,
  }) {
    return TransportState.newWithParam(
      isPlaying: isPlaying ?? this.isPlaying,
      isPatternPlaying: isPatternPlaying ?? this.isPatternPlaying,
      isRecording: isRecording ?? this.isRecording,
      isLooping: isLooping ?? this.isLooping,
      playheadPositionSamples:
          playheadPositionSamples ?? this.playheadPositionSamples,
      loopStartSamples: loopStartSamples ?? this.loopStartSamples,
      loopEndSamples: loopEndSamples ?? this.loopEndSamples,
      bpm: bpm ?? this.bpm,
      timeSignature: timeSignature ?? this.timeSignature,
      barTracker: barTracker ?? this.barTracker,
      beatTracker: beatTracker ?? this.beatTracker,
    );
  }
}
