// Custom Behavior to allow Mouse Dragging (Standard 'Inverse' Scrolling)
import 'dart:ui';

import 'package:flutter/material.dart';

class DragScrollBehavior extends MaterialScrollBehavior {
  @override
  Set<PointerDeviceKind> get dragDevices => {
    PointerDeviceKind.touch,
    PointerDeviceKind.mouse,
    PointerDeviceKind.trackpad,
  };
}