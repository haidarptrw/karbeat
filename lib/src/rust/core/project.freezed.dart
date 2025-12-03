// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'project.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$GeneratorInstance {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is GeneratorInstance);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'GeneratorInstance()';
}


}

/// @nodoc
class $GeneratorInstanceCopyWith<$Res>  {
$GeneratorInstanceCopyWith(GeneratorInstance _, $Res Function(GeneratorInstance) __);
}


/// Adds pattern-matching-related methods to [GeneratorInstance].
extension GeneratorInstancePatterns on GeneratorInstance {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( GeneratorInstance_Plugin value)?  plugin,TResult Function( GeneratorInstance_Sampler value)?  sampler,TResult Function( GeneratorInstance_AudioInput value)?  audioInput,required TResult orElse(),}){
final _that = this;
switch (_that) {
case GeneratorInstance_Plugin() when plugin != null:
return plugin(_that);case GeneratorInstance_Sampler() when sampler != null:
return sampler(_that);case GeneratorInstance_AudioInput() when audioInput != null:
return audioInput(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( GeneratorInstance_Plugin value)  plugin,required TResult Function( GeneratorInstance_Sampler value)  sampler,required TResult Function( GeneratorInstance_AudioInput value)  audioInput,}){
final _that = this;
switch (_that) {
case GeneratorInstance_Plugin():
return plugin(_that);case GeneratorInstance_Sampler():
return sampler(_that);case GeneratorInstance_AudioInput():
return audioInput(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( GeneratorInstance_Plugin value)?  plugin,TResult? Function( GeneratorInstance_Sampler value)?  sampler,TResult? Function( GeneratorInstance_AudioInput value)?  audioInput,}){
final _that = this;
switch (_that) {
case GeneratorInstance_Plugin() when plugin != null:
return plugin(_that);case GeneratorInstance_Sampler() when sampler != null:
return sampler(_that);case GeneratorInstance_AudioInput() when audioInput != null:
return audioInput(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( PluginInstance field0)?  plugin,TResult Function( int assetId,  int rootNote)?  sampler,TResult Function( int deviceChannelIndex)?  audioInput,required TResult orElse(),}) {final _that = this;
switch (_that) {
case GeneratorInstance_Plugin() when plugin != null:
return plugin(_that.field0);case GeneratorInstance_Sampler() when sampler != null:
return sampler(_that.assetId,_that.rootNote);case GeneratorInstance_AudioInput() when audioInput != null:
return audioInput(_that.deviceChannelIndex);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( PluginInstance field0)  plugin,required TResult Function( int assetId,  int rootNote)  sampler,required TResult Function( int deviceChannelIndex)  audioInput,}) {final _that = this;
switch (_that) {
case GeneratorInstance_Plugin():
return plugin(_that.field0);case GeneratorInstance_Sampler():
return sampler(_that.assetId,_that.rootNote);case GeneratorInstance_AudioInput():
return audioInput(_that.deviceChannelIndex);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( PluginInstance field0)?  plugin,TResult? Function( int assetId,  int rootNote)?  sampler,TResult? Function( int deviceChannelIndex)?  audioInput,}) {final _that = this;
switch (_that) {
case GeneratorInstance_Plugin() when plugin != null:
return plugin(_that.field0);case GeneratorInstance_Sampler() when sampler != null:
return sampler(_that.assetId,_that.rootNote);case GeneratorInstance_AudioInput() when audioInput != null:
return audioInput(_that.deviceChannelIndex);case _:
  return null;

}
}

}

/// @nodoc


class GeneratorInstance_Plugin extends GeneratorInstance {
  const GeneratorInstance_Plugin(this.field0): super._();
  

 final  PluginInstance field0;

/// Create a copy of GeneratorInstance
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GeneratorInstance_PluginCopyWith<GeneratorInstance_Plugin> get copyWith => _$GeneratorInstance_PluginCopyWithImpl<GeneratorInstance_Plugin>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is GeneratorInstance_Plugin&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'GeneratorInstance.plugin(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $GeneratorInstance_PluginCopyWith<$Res> implements $GeneratorInstanceCopyWith<$Res> {
  factory $GeneratorInstance_PluginCopyWith(GeneratorInstance_Plugin value, $Res Function(GeneratorInstance_Plugin) _then) = _$GeneratorInstance_PluginCopyWithImpl;
@useResult
$Res call({
 PluginInstance field0
});




}
/// @nodoc
class _$GeneratorInstance_PluginCopyWithImpl<$Res>
    implements $GeneratorInstance_PluginCopyWith<$Res> {
  _$GeneratorInstance_PluginCopyWithImpl(this._self, this._then);

  final GeneratorInstance_Plugin _self;
  final $Res Function(GeneratorInstance_Plugin) _then;

/// Create a copy of GeneratorInstance
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(GeneratorInstance_Plugin(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as PluginInstance,
  ));
}


}

/// @nodoc


class GeneratorInstance_Sampler extends GeneratorInstance {
  const GeneratorInstance_Sampler({required this.assetId, required this.rootNote}): super._();
  

 final  int assetId;
 final  int rootNote;

/// Create a copy of GeneratorInstance
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GeneratorInstance_SamplerCopyWith<GeneratorInstance_Sampler> get copyWith => _$GeneratorInstance_SamplerCopyWithImpl<GeneratorInstance_Sampler>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is GeneratorInstance_Sampler&&(identical(other.assetId, assetId) || other.assetId == assetId)&&(identical(other.rootNote, rootNote) || other.rootNote == rootNote));
}


@override
int get hashCode => Object.hash(runtimeType,assetId,rootNote);

@override
String toString() {
  return 'GeneratorInstance.sampler(assetId: $assetId, rootNote: $rootNote)';
}


}

/// @nodoc
abstract mixin class $GeneratorInstance_SamplerCopyWith<$Res> implements $GeneratorInstanceCopyWith<$Res> {
  factory $GeneratorInstance_SamplerCopyWith(GeneratorInstance_Sampler value, $Res Function(GeneratorInstance_Sampler) _then) = _$GeneratorInstance_SamplerCopyWithImpl;
@useResult
$Res call({
 int assetId, int rootNote
});




}
/// @nodoc
class _$GeneratorInstance_SamplerCopyWithImpl<$Res>
    implements $GeneratorInstance_SamplerCopyWith<$Res> {
  _$GeneratorInstance_SamplerCopyWithImpl(this._self, this._then);

  final GeneratorInstance_Sampler _self;
  final $Res Function(GeneratorInstance_Sampler) _then;

/// Create a copy of GeneratorInstance
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? assetId = null,Object? rootNote = null,}) {
  return _then(GeneratorInstance_Sampler(
assetId: null == assetId ? _self.assetId : assetId // ignore: cast_nullable_to_non_nullable
as int,rootNote: null == rootNote ? _self.rootNote : rootNote // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class GeneratorInstance_AudioInput extends GeneratorInstance {
  const GeneratorInstance_AudioInput({required this.deviceChannelIndex}): super._();
  

 final  int deviceChannelIndex;

/// Create a copy of GeneratorInstance
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GeneratorInstance_AudioInputCopyWith<GeneratorInstance_AudioInput> get copyWith => _$GeneratorInstance_AudioInputCopyWithImpl<GeneratorInstance_AudioInput>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is GeneratorInstance_AudioInput&&(identical(other.deviceChannelIndex, deviceChannelIndex) || other.deviceChannelIndex == deviceChannelIndex));
}


@override
int get hashCode => Object.hash(runtimeType,deviceChannelIndex);

@override
String toString() {
  return 'GeneratorInstance.audioInput(deviceChannelIndex: $deviceChannelIndex)';
}


}

/// @nodoc
abstract mixin class $GeneratorInstance_AudioInputCopyWith<$Res> implements $GeneratorInstanceCopyWith<$Res> {
  factory $GeneratorInstance_AudioInputCopyWith(GeneratorInstance_AudioInput value, $Res Function(GeneratorInstance_AudioInput) _then) = _$GeneratorInstance_AudioInputCopyWithImpl;
@useResult
$Res call({
 int deviceChannelIndex
});




}
/// @nodoc
class _$GeneratorInstance_AudioInputCopyWithImpl<$Res>
    implements $GeneratorInstance_AudioInputCopyWith<$Res> {
  _$GeneratorInstance_AudioInputCopyWithImpl(this._self, this._then);

  final GeneratorInstance_AudioInput _self;
  final $Res Function(GeneratorInstance_AudioInput) _then;

/// Create a copy of GeneratorInstance
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? deviceChannelIndex = null,}) {
  return _then(GeneratorInstance_AudioInput(
deviceChannelIndex: null == deviceChannelIndex ? _self.deviceChannelIndex : deviceChannelIndex // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc
mixin _$KarbeatSource {

 int get field0;
/// Create a copy of KarbeatSource
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$KarbeatSourceCopyWith<KarbeatSource> get copyWith => _$KarbeatSourceCopyWithImpl<KarbeatSource>(this as KarbeatSource, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is KarbeatSource&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'KarbeatSource(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $KarbeatSourceCopyWith<$Res>  {
  factory $KarbeatSourceCopyWith(KarbeatSource value, $Res Function(KarbeatSource) _then) = _$KarbeatSourceCopyWithImpl;
@useResult
$Res call({
 int field0
});




}
/// @nodoc
class _$KarbeatSourceCopyWithImpl<$Res>
    implements $KarbeatSourceCopyWith<$Res> {
  _$KarbeatSourceCopyWithImpl(this._self, this._then);

  final KarbeatSource _self;
  final $Res Function(KarbeatSource) _then;

/// Create a copy of KarbeatSource
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? field0 = null,}) {
  return _then(_self.copyWith(
field0: null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as int,
  ));
}

}


/// Adds pattern-matching-related methods to [KarbeatSource].
extension KarbeatSourcePatterns on KarbeatSource {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( KarbeatSource_Audio value)?  audio,TResult Function( KarbeatSource_Midi value)?  midi,TResult Function( KarbeatSource_Automation value)?  automation,required TResult orElse(),}){
final _that = this;
switch (_that) {
case KarbeatSource_Audio() when audio != null:
return audio(_that);case KarbeatSource_Midi() when midi != null:
return midi(_that);case KarbeatSource_Automation() when automation != null:
return automation(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( KarbeatSource_Audio value)  audio,required TResult Function( KarbeatSource_Midi value)  midi,required TResult Function( KarbeatSource_Automation value)  automation,}){
final _that = this;
switch (_that) {
case KarbeatSource_Audio():
return audio(_that);case KarbeatSource_Midi():
return midi(_that);case KarbeatSource_Automation():
return automation(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( KarbeatSource_Audio value)?  audio,TResult? Function( KarbeatSource_Midi value)?  midi,TResult? Function( KarbeatSource_Automation value)?  automation,}){
final _that = this;
switch (_that) {
case KarbeatSource_Audio() when audio != null:
return audio(_that);case KarbeatSource_Midi() when midi != null:
return midi(_that);case KarbeatSource_Automation() when automation != null:
return automation(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( int field0)?  audio,TResult Function( int field0)?  midi,TResult Function( int field0)?  automation,required TResult orElse(),}) {final _that = this;
switch (_that) {
case KarbeatSource_Audio() when audio != null:
return audio(_that.field0);case KarbeatSource_Midi() when midi != null:
return midi(_that.field0);case KarbeatSource_Automation() when automation != null:
return automation(_that.field0);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( int field0)  audio,required TResult Function( int field0)  midi,required TResult Function( int field0)  automation,}) {final _that = this;
switch (_that) {
case KarbeatSource_Audio():
return audio(_that.field0);case KarbeatSource_Midi():
return midi(_that.field0);case KarbeatSource_Automation():
return automation(_that.field0);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( int field0)?  audio,TResult? Function( int field0)?  midi,TResult? Function( int field0)?  automation,}) {final _that = this;
switch (_that) {
case KarbeatSource_Audio() when audio != null:
return audio(_that.field0);case KarbeatSource_Midi() when midi != null:
return midi(_that.field0);case KarbeatSource_Automation() when automation != null:
return automation(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class KarbeatSource_Audio extends KarbeatSource {
  const KarbeatSource_Audio(this.field0): super._();
  

@override final  int field0;

/// Create a copy of KarbeatSource
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$KarbeatSource_AudioCopyWith<KarbeatSource_Audio> get copyWith => _$KarbeatSource_AudioCopyWithImpl<KarbeatSource_Audio>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is KarbeatSource_Audio&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'KarbeatSource.audio(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $KarbeatSource_AudioCopyWith<$Res> implements $KarbeatSourceCopyWith<$Res> {
  factory $KarbeatSource_AudioCopyWith(KarbeatSource_Audio value, $Res Function(KarbeatSource_Audio) _then) = _$KarbeatSource_AudioCopyWithImpl;
@override @useResult
$Res call({
 int field0
});




}
/// @nodoc
class _$KarbeatSource_AudioCopyWithImpl<$Res>
    implements $KarbeatSource_AudioCopyWith<$Res> {
  _$KarbeatSource_AudioCopyWithImpl(this._self, this._then);

  final KarbeatSource_Audio _self;
  final $Res Function(KarbeatSource_Audio) _then;

/// Create a copy of KarbeatSource
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(KarbeatSource_Audio(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class KarbeatSource_Midi extends KarbeatSource {
  const KarbeatSource_Midi(this.field0): super._();
  

@override final  int field0;

/// Create a copy of KarbeatSource
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$KarbeatSource_MidiCopyWith<KarbeatSource_Midi> get copyWith => _$KarbeatSource_MidiCopyWithImpl<KarbeatSource_Midi>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is KarbeatSource_Midi&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'KarbeatSource.midi(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $KarbeatSource_MidiCopyWith<$Res> implements $KarbeatSourceCopyWith<$Res> {
  factory $KarbeatSource_MidiCopyWith(KarbeatSource_Midi value, $Res Function(KarbeatSource_Midi) _then) = _$KarbeatSource_MidiCopyWithImpl;
@override @useResult
$Res call({
 int field0
});




}
/// @nodoc
class _$KarbeatSource_MidiCopyWithImpl<$Res>
    implements $KarbeatSource_MidiCopyWith<$Res> {
  _$KarbeatSource_MidiCopyWithImpl(this._self, this._then);

  final KarbeatSource_Midi _self;
  final $Res Function(KarbeatSource_Midi) _then;

/// Create a copy of KarbeatSource
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(KarbeatSource_Midi(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class KarbeatSource_Automation extends KarbeatSource {
  const KarbeatSource_Automation(this.field0): super._();
  

@override final  int field0;

/// Create a copy of KarbeatSource
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$KarbeatSource_AutomationCopyWith<KarbeatSource_Automation> get copyWith => _$KarbeatSource_AutomationCopyWithImpl<KarbeatSource_Automation>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is KarbeatSource_Automation&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'KarbeatSource.automation(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $KarbeatSource_AutomationCopyWith<$Res> implements $KarbeatSourceCopyWith<$Res> {
  factory $KarbeatSource_AutomationCopyWith(KarbeatSource_Automation value, $Res Function(KarbeatSource_Automation) _then) = _$KarbeatSource_AutomationCopyWithImpl;
@override @useResult
$Res call({
 int field0
});




}
/// @nodoc
class _$KarbeatSource_AutomationCopyWithImpl<$Res>
    implements $KarbeatSource_AutomationCopyWith<$Res> {
  _$KarbeatSource_AutomationCopyWithImpl(this._self, this._then);

  final KarbeatSource_Automation _self;
  final $Res Function(KarbeatSource_Automation) _then;

/// Create a copy of KarbeatSource
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(KarbeatSource_Automation(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

// dart format on
