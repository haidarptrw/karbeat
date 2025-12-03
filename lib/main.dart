import 'package:flutter/material.dart';
import 'package:karbeat/app.dart';
import 'package:karbeat/src/rust/frb_generated.dart';
import 'package:karbeat/state/app_state.dart';
import 'package:provider/provider.dart';

Future<void> main() async {
  await RustLib.init();

  runApp(
    MultiProvider(
      providers: [ChangeNotifierProvider(create: (_) => KarbeatState())],
      child: const KarbeatApp(),
    ),
  );
}
