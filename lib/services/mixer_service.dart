import 'package:karbeat/src/rust/api/mixer.dart';
import 'package:karbeat/state/app_state.dart';
import 'package:karbeat/utils/result_type.dart';

List<UiRoutingConnection> getMixerChannelDest(
  GlobalAppState state, {
  required UiRoutingNode source,
}) {
  // find from the mixer state and return it. We don't fetch
  // the latest state from Rust side because we assume a guarantee that every
  // routing data update syncs the state from backend to the Flutter side
  return state.mixerState.routing.where((conn) {
    return conn.source == source;
  }).toList();
}

Future<Result<void>> addNewRouting(
  GlobalAppState state, {
  required UiRoutingNode source,
  required UiRoutingNode dest,
  required double sendLevel,
  required bool isSend,
}) async {
  return await attemptAsync(() async {
    await setRouting(
      source: source,
      destination: dest,
      sendLevel: sendLevel,
      isSend: isSend,
    );

    // Attempt a sync only on routing connection
    
  });
}
