// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'automation.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$AutomationTargetDto {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AutomationTargetDto);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'AutomationTargetDto()';
}


}

/// @nodoc
class $AutomationTargetDtoCopyWith<$Res>  {
$AutomationTargetDtoCopyWith(AutomationTargetDto _, $Res Function(AutomationTargetDto) __);
}


/// Adds pattern-matching-related methods to [AutomationTargetDto].
extension AutomationTargetDtoPatterns on AutomationTargetDto {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( AutomationTargetDto_Track value)?  track,TResult Function( AutomationTargetDto_Bus value)?  bus,TResult Function( AutomationTargetDto_Master value)?  master,TResult Function( AutomationTargetDto_TempoBpm value)?  tempoBpm,required TResult orElse(),}){
final _that = this;
switch (_that) {
case AutomationTargetDto_Track() when track != null:
return track(_that);case AutomationTargetDto_Bus() when bus != null:
return bus(_that);case AutomationTargetDto_Master() when master != null:
return master(_that);case AutomationTargetDto_TempoBpm() when tempoBpm != null:
return tempoBpm(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( AutomationTargetDto_Track value)  track,required TResult Function( AutomationTargetDto_Bus value)  bus,required TResult Function( AutomationTargetDto_Master value)  master,required TResult Function( AutomationTargetDto_TempoBpm value)  tempoBpm,}){
final _that = this;
switch (_that) {
case AutomationTargetDto_Track():
return track(_that);case AutomationTargetDto_Bus():
return bus(_that);case AutomationTargetDto_Master():
return master(_that);case AutomationTargetDto_TempoBpm():
return tempoBpm(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( AutomationTargetDto_Track value)?  track,TResult? Function( AutomationTargetDto_Bus value)?  bus,TResult? Function( AutomationTargetDto_Master value)?  master,TResult? Function( AutomationTargetDto_TempoBpm value)?  tempoBpm,}){
final _that = this;
switch (_that) {
case AutomationTargetDto_Track() when track != null:
return track(_that);case AutomationTargetDto_Bus() when bus != null:
return bus(_that);case AutomationTargetDto_Master() when master != null:
return master(_that);case AutomationTargetDto_TempoBpm() when tempoBpm != null:
return tempoBpm(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( int trackId,  TrackAutomationTargetDto trackTarget)?  track,TResult Function( int busId,  MixerChannelParamTargetDto mixTarget)?  bus,TResult Function( MixerChannelParamTargetDto field0)?  master,TResult Function()?  tempoBpm,required TResult orElse(),}) {final _that = this;
switch (_that) {
case AutomationTargetDto_Track() when track != null:
return track(_that.trackId,_that.trackTarget);case AutomationTargetDto_Bus() when bus != null:
return bus(_that.busId,_that.mixTarget);case AutomationTargetDto_Master() when master != null:
return master(_that.field0);case AutomationTargetDto_TempoBpm() when tempoBpm != null:
return tempoBpm();case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( int trackId,  TrackAutomationTargetDto trackTarget)  track,required TResult Function( int busId,  MixerChannelParamTargetDto mixTarget)  bus,required TResult Function( MixerChannelParamTargetDto field0)  master,required TResult Function()  tempoBpm,}) {final _that = this;
switch (_that) {
case AutomationTargetDto_Track():
return track(_that.trackId,_that.trackTarget);case AutomationTargetDto_Bus():
return bus(_that.busId,_that.mixTarget);case AutomationTargetDto_Master():
return master(_that.field0);case AutomationTargetDto_TempoBpm():
return tempoBpm();}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( int trackId,  TrackAutomationTargetDto trackTarget)?  track,TResult? Function( int busId,  MixerChannelParamTargetDto mixTarget)?  bus,TResult? Function( MixerChannelParamTargetDto field0)?  master,TResult? Function()?  tempoBpm,}) {final _that = this;
switch (_that) {
case AutomationTargetDto_Track() when track != null:
return track(_that.trackId,_that.trackTarget);case AutomationTargetDto_Bus() when bus != null:
return bus(_that.busId,_that.mixTarget);case AutomationTargetDto_Master() when master != null:
return master(_that.field0);case AutomationTargetDto_TempoBpm() when tempoBpm != null:
return tempoBpm();case _:
  return null;

}
}

}

/// @nodoc


class AutomationTargetDto_Track extends AutomationTargetDto {
  const AutomationTargetDto_Track({required this.trackId, required this.trackTarget}): super._();
  

 final  int trackId;
 final  TrackAutomationTargetDto trackTarget;

/// Create a copy of AutomationTargetDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$AutomationTargetDto_TrackCopyWith<AutomationTargetDto_Track> get copyWith => _$AutomationTargetDto_TrackCopyWithImpl<AutomationTargetDto_Track>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AutomationTargetDto_Track&&(identical(other.trackId, trackId) || other.trackId == trackId)&&(identical(other.trackTarget, trackTarget) || other.trackTarget == trackTarget));
}


@override
int get hashCode => Object.hash(runtimeType,trackId,trackTarget);

@override
String toString() {
  return 'AutomationTargetDto.track(trackId: $trackId, trackTarget: $trackTarget)';
}


}

/// @nodoc
abstract mixin class $AutomationTargetDto_TrackCopyWith<$Res> implements $AutomationTargetDtoCopyWith<$Res> {
  factory $AutomationTargetDto_TrackCopyWith(AutomationTargetDto_Track value, $Res Function(AutomationTargetDto_Track) _then) = _$AutomationTargetDto_TrackCopyWithImpl;
@useResult
$Res call({
 int trackId, TrackAutomationTargetDto trackTarget
});


$TrackAutomationTargetDtoCopyWith<$Res> get trackTarget;

}
/// @nodoc
class _$AutomationTargetDto_TrackCopyWithImpl<$Res>
    implements $AutomationTargetDto_TrackCopyWith<$Res> {
  _$AutomationTargetDto_TrackCopyWithImpl(this._self, this._then);

  final AutomationTargetDto_Track _self;
  final $Res Function(AutomationTargetDto_Track) _then;

/// Create a copy of AutomationTargetDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? trackId = null,Object? trackTarget = null,}) {
  return _then(AutomationTargetDto_Track(
trackId: null == trackId ? _self.trackId : trackId // ignore: cast_nullable_to_non_nullable
as int,trackTarget: null == trackTarget ? _self.trackTarget : trackTarget // ignore: cast_nullable_to_non_nullable
as TrackAutomationTargetDto,
  ));
}

/// Create a copy of AutomationTargetDto
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$TrackAutomationTargetDtoCopyWith<$Res> get trackTarget {
  
  return $TrackAutomationTargetDtoCopyWith<$Res>(_self.trackTarget, (value) {
    return _then(_self.copyWith(trackTarget: value));
  });
}
}

/// @nodoc


class AutomationTargetDto_Bus extends AutomationTargetDto {
  const AutomationTargetDto_Bus({required this.busId, required this.mixTarget}): super._();
  

 final  int busId;
 final  MixerChannelParamTargetDto mixTarget;

/// Create a copy of AutomationTargetDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$AutomationTargetDto_BusCopyWith<AutomationTargetDto_Bus> get copyWith => _$AutomationTargetDto_BusCopyWithImpl<AutomationTargetDto_Bus>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AutomationTargetDto_Bus&&(identical(other.busId, busId) || other.busId == busId)&&(identical(other.mixTarget, mixTarget) || other.mixTarget == mixTarget));
}


@override
int get hashCode => Object.hash(runtimeType,busId,mixTarget);

@override
String toString() {
  return 'AutomationTargetDto.bus(busId: $busId, mixTarget: $mixTarget)';
}


}

/// @nodoc
abstract mixin class $AutomationTargetDto_BusCopyWith<$Res> implements $AutomationTargetDtoCopyWith<$Res> {
  factory $AutomationTargetDto_BusCopyWith(AutomationTargetDto_Bus value, $Res Function(AutomationTargetDto_Bus) _then) = _$AutomationTargetDto_BusCopyWithImpl;
@useResult
$Res call({
 int busId, MixerChannelParamTargetDto mixTarget
});


$MixerChannelParamTargetDtoCopyWith<$Res> get mixTarget;

}
/// @nodoc
class _$AutomationTargetDto_BusCopyWithImpl<$Res>
    implements $AutomationTargetDto_BusCopyWith<$Res> {
  _$AutomationTargetDto_BusCopyWithImpl(this._self, this._then);

  final AutomationTargetDto_Bus _self;
  final $Res Function(AutomationTargetDto_Bus) _then;

/// Create a copy of AutomationTargetDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? busId = null,Object? mixTarget = null,}) {
  return _then(AutomationTargetDto_Bus(
busId: null == busId ? _self.busId : busId // ignore: cast_nullable_to_non_nullable
as int,mixTarget: null == mixTarget ? _self.mixTarget : mixTarget // ignore: cast_nullable_to_non_nullable
as MixerChannelParamTargetDto,
  ));
}

/// Create a copy of AutomationTargetDto
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$MixerChannelParamTargetDtoCopyWith<$Res> get mixTarget {
  
  return $MixerChannelParamTargetDtoCopyWith<$Res>(_self.mixTarget, (value) {
    return _then(_self.copyWith(mixTarget: value));
  });
}
}

/// @nodoc


class AutomationTargetDto_Master extends AutomationTargetDto {
  const AutomationTargetDto_Master(this.field0): super._();
  

 final  MixerChannelParamTargetDto field0;

/// Create a copy of AutomationTargetDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$AutomationTargetDto_MasterCopyWith<AutomationTargetDto_Master> get copyWith => _$AutomationTargetDto_MasterCopyWithImpl<AutomationTargetDto_Master>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AutomationTargetDto_Master&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'AutomationTargetDto.master(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $AutomationTargetDto_MasterCopyWith<$Res> implements $AutomationTargetDtoCopyWith<$Res> {
  factory $AutomationTargetDto_MasterCopyWith(AutomationTargetDto_Master value, $Res Function(AutomationTargetDto_Master) _then) = _$AutomationTargetDto_MasterCopyWithImpl;
@useResult
$Res call({
 MixerChannelParamTargetDto field0
});


$MixerChannelParamTargetDtoCopyWith<$Res> get field0;

}
/// @nodoc
class _$AutomationTargetDto_MasterCopyWithImpl<$Res>
    implements $AutomationTargetDto_MasterCopyWith<$Res> {
  _$AutomationTargetDto_MasterCopyWithImpl(this._self, this._then);

  final AutomationTargetDto_Master _self;
  final $Res Function(AutomationTargetDto_Master) _then;

/// Create a copy of AutomationTargetDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(AutomationTargetDto_Master(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as MixerChannelParamTargetDto,
  ));
}

/// Create a copy of AutomationTargetDto
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$MixerChannelParamTargetDtoCopyWith<$Res> get field0 {
  
  return $MixerChannelParamTargetDtoCopyWith<$Res>(_self.field0, (value) {
    return _then(_self.copyWith(field0: value));
  });
}
}

/// @nodoc


class AutomationTargetDto_TempoBpm extends AutomationTargetDto {
  const AutomationTargetDto_TempoBpm(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AutomationTargetDto_TempoBpm);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'AutomationTargetDto.tempoBpm()';
}


}




/// @nodoc
mixin _$EffectAutomationTargetDto {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is EffectAutomationTargetDto);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'EffectAutomationTargetDto()';
}


}

/// @nodoc
class $EffectAutomationTargetDtoCopyWith<$Res>  {
$EffectAutomationTargetDtoCopyWith(EffectAutomationTargetDto _, $Res Function(EffectAutomationTargetDto) __);
}


/// Adds pattern-matching-related methods to [EffectAutomationTargetDto].
extension EffectAutomationTargetDtoPatterns on EffectAutomationTargetDto {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( EffectAutomationTargetDto_Mix value)?  mix,TResult Function( EffectAutomationTargetDto_PluginParam value)?  pluginParam,required TResult orElse(),}){
final _that = this;
switch (_that) {
case EffectAutomationTargetDto_Mix() when mix != null:
return mix(_that);case EffectAutomationTargetDto_PluginParam() when pluginParam != null:
return pluginParam(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( EffectAutomationTargetDto_Mix value)  mix,required TResult Function( EffectAutomationTargetDto_PluginParam value)  pluginParam,}){
final _that = this;
switch (_that) {
case EffectAutomationTargetDto_Mix():
return mix(_that);case EffectAutomationTargetDto_PluginParam():
return pluginParam(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( EffectAutomationTargetDto_Mix value)?  mix,TResult? Function( EffectAutomationTargetDto_PluginParam value)?  pluginParam,}){
final _that = this;
switch (_that) {
case EffectAutomationTargetDto_Mix() when mix != null:
return mix(_that);case EffectAutomationTargetDto_PluginParam() when pluginParam != null:
return pluginParam(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function()?  mix,TResult Function( int paramId)?  pluginParam,required TResult orElse(),}) {final _that = this;
switch (_that) {
case EffectAutomationTargetDto_Mix() when mix != null:
return mix();case EffectAutomationTargetDto_PluginParam() when pluginParam != null:
return pluginParam(_that.paramId);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function()  mix,required TResult Function( int paramId)  pluginParam,}) {final _that = this;
switch (_that) {
case EffectAutomationTargetDto_Mix():
return mix();case EffectAutomationTargetDto_PluginParam():
return pluginParam(_that.paramId);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function()?  mix,TResult? Function( int paramId)?  pluginParam,}) {final _that = this;
switch (_that) {
case EffectAutomationTargetDto_Mix() when mix != null:
return mix();case EffectAutomationTargetDto_PluginParam() when pluginParam != null:
return pluginParam(_that.paramId);case _:
  return null;

}
}

}

/// @nodoc


class EffectAutomationTargetDto_Mix extends EffectAutomationTargetDto {
  const EffectAutomationTargetDto_Mix(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is EffectAutomationTargetDto_Mix);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'EffectAutomationTargetDto.mix()';
}


}




/// @nodoc


class EffectAutomationTargetDto_PluginParam extends EffectAutomationTargetDto {
  const EffectAutomationTargetDto_PluginParam({required this.paramId}): super._();
  

 final  int paramId;

/// Create a copy of EffectAutomationTargetDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$EffectAutomationTargetDto_PluginParamCopyWith<EffectAutomationTargetDto_PluginParam> get copyWith => _$EffectAutomationTargetDto_PluginParamCopyWithImpl<EffectAutomationTargetDto_PluginParam>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is EffectAutomationTargetDto_PluginParam&&(identical(other.paramId, paramId) || other.paramId == paramId));
}


@override
int get hashCode => Object.hash(runtimeType,paramId);

@override
String toString() {
  return 'EffectAutomationTargetDto.pluginParam(paramId: $paramId)';
}


}

/// @nodoc
abstract mixin class $EffectAutomationTargetDto_PluginParamCopyWith<$Res> implements $EffectAutomationTargetDtoCopyWith<$Res> {
  factory $EffectAutomationTargetDto_PluginParamCopyWith(EffectAutomationTargetDto_PluginParam value, $Res Function(EffectAutomationTargetDto_PluginParam) _then) = _$EffectAutomationTargetDto_PluginParamCopyWithImpl;
@useResult
$Res call({
 int paramId
});




}
/// @nodoc
class _$EffectAutomationTargetDto_PluginParamCopyWithImpl<$Res>
    implements $EffectAutomationTargetDto_PluginParamCopyWith<$Res> {
  _$EffectAutomationTargetDto_PluginParamCopyWithImpl(this._self, this._then);

  final EffectAutomationTargetDto_PluginParam _self;
  final $Res Function(EffectAutomationTargetDto_PluginParam) _then;

/// Create a copy of EffectAutomationTargetDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? paramId = null,}) {
  return _then(EffectAutomationTargetDto_PluginParam(
paramId: null == paramId ? _self.paramId : paramId // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc
mixin _$MixerChannelParamTargetDto {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is MixerChannelParamTargetDto);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'MixerChannelParamTargetDto()';
}


}

/// @nodoc
class $MixerChannelParamTargetDtoCopyWith<$Res>  {
$MixerChannelParamTargetDtoCopyWith(MixerChannelParamTargetDto _, $Res Function(MixerChannelParamTargetDto) __);
}


/// Adds pattern-matching-related methods to [MixerChannelParamTargetDto].
extension MixerChannelParamTargetDtoPatterns on MixerChannelParamTargetDto {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( MixerChannelParamTargetDto_Volume value)?  volume,TResult Function( MixerChannelParamTargetDto_Pan value)?  pan,TResult Function( MixerChannelParamTargetDto_Plugin value)?  plugin,required TResult orElse(),}){
final _that = this;
switch (_that) {
case MixerChannelParamTargetDto_Volume() when volume != null:
return volume(_that);case MixerChannelParamTargetDto_Pan() when pan != null:
return pan(_that);case MixerChannelParamTargetDto_Plugin() when plugin != null:
return plugin(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( MixerChannelParamTargetDto_Volume value)  volume,required TResult Function( MixerChannelParamTargetDto_Pan value)  pan,required TResult Function( MixerChannelParamTargetDto_Plugin value)  plugin,}){
final _that = this;
switch (_that) {
case MixerChannelParamTargetDto_Volume():
return volume(_that);case MixerChannelParamTargetDto_Pan():
return pan(_that);case MixerChannelParamTargetDto_Plugin():
return plugin(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( MixerChannelParamTargetDto_Volume value)?  volume,TResult? Function( MixerChannelParamTargetDto_Pan value)?  pan,TResult? Function( MixerChannelParamTargetDto_Plugin value)?  plugin,}){
final _that = this;
switch (_that) {
case MixerChannelParamTargetDto_Volume() when volume != null:
return volume(_that);case MixerChannelParamTargetDto_Pan() when pan != null:
return pan(_that);case MixerChannelParamTargetDto_Plugin() when plugin != null:
return plugin(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function()?  volume,TResult Function()?  pan,TResult Function( int effectId,  EffectAutomationTargetDto target)?  plugin,required TResult orElse(),}) {final _that = this;
switch (_that) {
case MixerChannelParamTargetDto_Volume() when volume != null:
return volume();case MixerChannelParamTargetDto_Pan() when pan != null:
return pan();case MixerChannelParamTargetDto_Plugin() when plugin != null:
return plugin(_that.effectId,_that.target);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function()  volume,required TResult Function()  pan,required TResult Function( int effectId,  EffectAutomationTargetDto target)  plugin,}) {final _that = this;
switch (_that) {
case MixerChannelParamTargetDto_Volume():
return volume();case MixerChannelParamTargetDto_Pan():
return pan();case MixerChannelParamTargetDto_Plugin():
return plugin(_that.effectId,_that.target);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function()?  volume,TResult? Function()?  pan,TResult? Function( int effectId,  EffectAutomationTargetDto target)?  plugin,}) {final _that = this;
switch (_that) {
case MixerChannelParamTargetDto_Volume() when volume != null:
return volume();case MixerChannelParamTargetDto_Pan() when pan != null:
return pan();case MixerChannelParamTargetDto_Plugin() when plugin != null:
return plugin(_that.effectId,_that.target);case _:
  return null;

}
}

}

/// @nodoc


class MixerChannelParamTargetDto_Volume extends MixerChannelParamTargetDto {
  const MixerChannelParamTargetDto_Volume(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is MixerChannelParamTargetDto_Volume);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'MixerChannelParamTargetDto.volume()';
}


}




/// @nodoc


class MixerChannelParamTargetDto_Pan extends MixerChannelParamTargetDto {
  const MixerChannelParamTargetDto_Pan(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is MixerChannelParamTargetDto_Pan);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'MixerChannelParamTargetDto.pan()';
}


}




/// @nodoc


class MixerChannelParamTargetDto_Plugin extends MixerChannelParamTargetDto {
  const MixerChannelParamTargetDto_Plugin({required this.effectId, required this.target}): super._();
  

 final  int effectId;
 final  EffectAutomationTargetDto target;

/// Create a copy of MixerChannelParamTargetDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$MixerChannelParamTargetDto_PluginCopyWith<MixerChannelParamTargetDto_Plugin> get copyWith => _$MixerChannelParamTargetDto_PluginCopyWithImpl<MixerChannelParamTargetDto_Plugin>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is MixerChannelParamTargetDto_Plugin&&(identical(other.effectId, effectId) || other.effectId == effectId)&&(identical(other.target, target) || other.target == target));
}


@override
int get hashCode => Object.hash(runtimeType,effectId,target);

@override
String toString() {
  return 'MixerChannelParamTargetDto.plugin(effectId: $effectId, target: $target)';
}


}

/// @nodoc
abstract mixin class $MixerChannelParamTargetDto_PluginCopyWith<$Res> implements $MixerChannelParamTargetDtoCopyWith<$Res> {
  factory $MixerChannelParamTargetDto_PluginCopyWith(MixerChannelParamTargetDto_Plugin value, $Res Function(MixerChannelParamTargetDto_Plugin) _then) = _$MixerChannelParamTargetDto_PluginCopyWithImpl;
@useResult
$Res call({
 int effectId, EffectAutomationTargetDto target
});


$EffectAutomationTargetDtoCopyWith<$Res> get target;

}
/// @nodoc
class _$MixerChannelParamTargetDto_PluginCopyWithImpl<$Res>
    implements $MixerChannelParamTargetDto_PluginCopyWith<$Res> {
  _$MixerChannelParamTargetDto_PluginCopyWithImpl(this._self, this._then);

  final MixerChannelParamTargetDto_Plugin _self;
  final $Res Function(MixerChannelParamTargetDto_Plugin) _then;

/// Create a copy of MixerChannelParamTargetDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? effectId = null,Object? target = null,}) {
  return _then(MixerChannelParamTargetDto_Plugin(
effectId: null == effectId ? _self.effectId : effectId // ignore: cast_nullable_to_non_nullable
as int,target: null == target ? _self.target : target // ignore: cast_nullable_to_non_nullable
as EffectAutomationTargetDto,
  ));
}

/// Create a copy of MixerChannelParamTargetDto
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$EffectAutomationTargetDtoCopyWith<$Res> get target {
  
  return $EffectAutomationTargetDtoCopyWith<$Res>(_self.target, (value) {
    return _then(_self.copyWith(target: value));
  });
}
}

/// @nodoc
mixin _$ModulationSourceDto {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ModulationSourceDto);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'ModulationSourceDto()';
}


}

/// @nodoc
class $ModulationSourceDtoCopyWith<$Res>  {
$ModulationSourceDtoCopyWith(ModulationSourceDto _, $Res Function(ModulationSourceDto) __);
}


/// Adds pattern-matching-related methods to [ModulationSourceDto].
extension ModulationSourceDtoPatterns on ModulationSourceDto {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( ModulationSourceDto_PeakController value)?  peakController,TResult Function( ModulationSourceDto_Automation value)?  automation,TResult Function( ModulationSourceDto_Lfo value)?  lfo,required TResult orElse(),}){
final _that = this;
switch (_that) {
case ModulationSourceDto_PeakController() when peakController != null:
return peakController(_that);case ModulationSourceDto_Automation() when automation != null:
return automation(_that);case ModulationSourceDto_Lfo() when lfo != null:
return lfo(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( ModulationSourceDto_PeakController value)  peakController,required TResult Function( ModulationSourceDto_Automation value)  automation,required TResult Function( ModulationSourceDto_Lfo value)  lfo,}){
final _that = this;
switch (_that) {
case ModulationSourceDto_PeakController():
return peakController(_that);case ModulationSourceDto_Automation():
return automation(_that);case ModulationSourceDto_Lfo():
return lfo(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( ModulationSourceDto_PeakController value)?  peakController,TResult? Function( ModulationSourceDto_Automation value)?  automation,TResult? Function( ModulationSourceDto_Lfo value)?  lfo,}){
final _that = this;
switch (_that) {
case ModulationSourceDto_PeakController() when peakController != null:
return peakController(_that);case ModulationSourceDto_Automation() when automation != null:
return automation(_that);case ModulationSourceDto_Lfo() when lfo != null:
return lfo(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( UiPluginTarget source)?  peakController,TResult Function( int laneId)?  automation,TResult Function( double rateHz)?  lfo,required TResult orElse(),}) {final _that = this;
switch (_that) {
case ModulationSourceDto_PeakController() when peakController != null:
return peakController(_that.source);case ModulationSourceDto_Automation() when automation != null:
return automation(_that.laneId);case ModulationSourceDto_Lfo() when lfo != null:
return lfo(_that.rateHz);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( UiPluginTarget source)  peakController,required TResult Function( int laneId)  automation,required TResult Function( double rateHz)  lfo,}) {final _that = this;
switch (_that) {
case ModulationSourceDto_PeakController():
return peakController(_that.source);case ModulationSourceDto_Automation():
return automation(_that.laneId);case ModulationSourceDto_Lfo():
return lfo(_that.rateHz);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( UiPluginTarget source)?  peakController,TResult? Function( int laneId)?  automation,TResult? Function( double rateHz)?  lfo,}) {final _that = this;
switch (_that) {
case ModulationSourceDto_PeakController() when peakController != null:
return peakController(_that.source);case ModulationSourceDto_Automation() when automation != null:
return automation(_that.laneId);case ModulationSourceDto_Lfo() when lfo != null:
return lfo(_that.rateHz);case _:
  return null;

}
}

}

/// @nodoc


class ModulationSourceDto_PeakController extends ModulationSourceDto {
  const ModulationSourceDto_PeakController({required this.source}): super._();
  

 final  UiPluginTarget source;

/// Create a copy of ModulationSourceDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ModulationSourceDto_PeakControllerCopyWith<ModulationSourceDto_PeakController> get copyWith => _$ModulationSourceDto_PeakControllerCopyWithImpl<ModulationSourceDto_PeakController>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ModulationSourceDto_PeakController&&(identical(other.source, source) || other.source == source));
}


@override
int get hashCode => Object.hash(runtimeType,source);

@override
String toString() {
  return 'ModulationSourceDto.peakController(source: $source)';
}


}

/// @nodoc
abstract mixin class $ModulationSourceDto_PeakControllerCopyWith<$Res> implements $ModulationSourceDtoCopyWith<$Res> {
  factory $ModulationSourceDto_PeakControllerCopyWith(ModulationSourceDto_PeakController value, $Res Function(ModulationSourceDto_PeakController) _then) = _$ModulationSourceDto_PeakControllerCopyWithImpl;
@useResult
$Res call({
 UiPluginTarget source
});


$UiPluginTargetCopyWith<$Res> get source;

}
/// @nodoc
class _$ModulationSourceDto_PeakControllerCopyWithImpl<$Res>
    implements $ModulationSourceDto_PeakControllerCopyWith<$Res> {
  _$ModulationSourceDto_PeakControllerCopyWithImpl(this._self, this._then);

  final ModulationSourceDto_PeakController _self;
  final $Res Function(ModulationSourceDto_PeakController) _then;

/// Create a copy of ModulationSourceDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? source = null,}) {
  return _then(ModulationSourceDto_PeakController(
source: null == source ? _self.source : source // ignore: cast_nullable_to_non_nullable
as UiPluginTarget,
  ));
}

/// Create a copy of ModulationSourceDto
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$UiPluginTargetCopyWith<$Res> get source {
  
  return $UiPluginTargetCopyWith<$Res>(_self.source, (value) {
    return _then(_self.copyWith(source: value));
  });
}
}

/// @nodoc


class ModulationSourceDto_Automation extends ModulationSourceDto {
  const ModulationSourceDto_Automation({required this.laneId}): super._();
  

 final  int laneId;

/// Create a copy of ModulationSourceDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ModulationSourceDto_AutomationCopyWith<ModulationSourceDto_Automation> get copyWith => _$ModulationSourceDto_AutomationCopyWithImpl<ModulationSourceDto_Automation>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ModulationSourceDto_Automation&&(identical(other.laneId, laneId) || other.laneId == laneId));
}


@override
int get hashCode => Object.hash(runtimeType,laneId);

@override
String toString() {
  return 'ModulationSourceDto.automation(laneId: $laneId)';
}


}

/// @nodoc
abstract mixin class $ModulationSourceDto_AutomationCopyWith<$Res> implements $ModulationSourceDtoCopyWith<$Res> {
  factory $ModulationSourceDto_AutomationCopyWith(ModulationSourceDto_Automation value, $Res Function(ModulationSourceDto_Automation) _then) = _$ModulationSourceDto_AutomationCopyWithImpl;
@useResult
$Res call({
 int laneId
});




}
/// @nodoc
class _$ModulationSourceDto_AutomationCopyWithImpl<$Res>
    implements $ModulationSourceDto_AutomationCopyWith<$Res> {
  _$ModulationSourceDto_AutomationCopyWithImpl(this._self, this._then);

  final ModulationSourceDto_Automation _self;
  final $Res Function(ModulationSourceDto_Automation) _then;

/// Create a copy of ModulationSourceDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? laneId = null,}) {
  return _then(ModulationSourceDto_Automation(
laneId: null == laneId ? _self.laneId : laneId // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class ModulationSourceDto_Lfo extends ModulationSourceDto {
  const ModulationSourceDto_Lfo({required this.rateHz}): super._();
  

 final  double rateHz;

/// Create a copy of ModulationSourceDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ModulationSourceDto_LfoCopyWith<ModulationSourceDto_Lfo> get copyWith => _$ModulationSourceDto_LfoCopyWithImpl<ModulationSourceDto_Lfo>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ModulationSourceDto_Lfo&&(identical(other.rateHz, rateHz) || other.rateHz == rateHz));
}


@override
int get hashCode => Object.hash(runtimeType,rateHz);

@override
String toString() {
  return 'ModulationSourceDto.lfo(rateHz: $rateHz)';
}


}

/// @nodoc
abstract mixin class $ModulationSourceDto_LfoCopyWith<$Res> implements $ModulationSourceDtoCopyWith<$Res> {
  factory $ModulationSourceDto_LfoCopyWith(ModulationSourceDto_Lfo value, $Res Function(ModulationSourceDto_Lfo) _then) = _$ModulationSourceDto_LfoCopyWithImpl;
@useResult
$Res call({
 double rateHz
});




}
/// @nodoc
class _$ModulationSourceDto_LfoCopyWithImpl<$Res>
    implements $ModulationSourceDto_LfoCopyWith<$Res> {
  _$ModulationSourceDto_LfoCopyWithImpl(this._self, this._then);

  final ModulationSourceDto_Lfo _self;
  final $Res Function(ModulationSourceDto_Lfo) _then;

/// Create a copy of ModulationSourceDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? rateHz = null,}) {
  return _then(ModulationSourceDto_Lfo(
rateHz: null == rateHz ? _self.rateHz : rateHz // ignore: cast_nullable_to_non_nullable
as double,
  ));
}


}

/// @nodoc
mixin _$TrackAutomationTargetDto {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TrackAutomationTargetDto);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'TrackAutomationTargetDto()';
}


}

/// @nodoc
class $TrackAutomationTargetDtoCopyWith<$Res>  {
$TrackAutomationTargetDtoCopyWith(TrackAutomationTargetDto _, $Res Function(TrackAutomationTargetDto) __);
}


/// Adds pattern-matching-related methods to [TrackAutomationTargetDto].
extension TrackAutomationTargetDtoPatterns on TrackAutomationTargetDto {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( TrackAutomationTargetDto_Generator value)?  generator,TResult Function( TrackAutomationTargetDto_MixerChannel value)?  mixerChannel,required TResult orElse(),}){
final _that = this;
switch (_that) {
case TrackAutomationTargetDto_Generator() when generator != null:
return generator(_that);case TrackAutomationTargetDto_MixerChannel() when mixerChannel != null:
return mixerChannel(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( TrackAutomationTargetDto_Generator value)  generator,required TResult Function( TrackAutomationTargetDto_MixerChannel value)  mixerChannel,}){
final _that = this;
switch (_that) {
case TrackAutomationTargetDto_Generator():
return generator(_that);case TrackAutomationTargetDto_MixerChannel():
return mixerChannel(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( TrackAutomationTargetDto_Generator value)?  generator,TResult? Function( TrackAutomationTargetDto_MixerChannel value)?  mixerChannel,}){
final _that = this;
switch (_that) {
case TrackAutomationTargetDto_Generator() when generator != null:
return generator(_that);case TrackAutomationTargetDto_MixerChannel() when mixerChannel != null:
return mixerChannel(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( int paramId)?  generator,TResult Function( MixerChannelParamTargetDto field0)?  mixerChannel,required TResult orElse(),}) {final _that = this;
switch (_that) {
case TrackAutomationTargetDto_Generator() when generator != null:
return generator(_that.paramId);case TrackAutomationTargetDto_MixerChannel() when mixerChannel != null:
return mixerChannel(_that.field0);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( int paramId)  generator,required TResult Function( MixerChannelParamTargetDto field0)  mixerChannel,}) {final _that = this;
switch (_that) {
case TrackAutomationTargetDto_Generator():
return generator(_that.paramId);case TrackAutomationTargetDto_MixerChannel():
return mixerChannel(_that.field0);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( int paramId)?  generator,TResult? Function( MixerChannelParamTargetDto field0)?  mixerChannel,}) {final _that = this;
switch (_that) {
case TrackAutomationTargetDto_Generator() when generator != null:
return generator(_that.paramId);case TrackAutomationTargetDto_MixerChannel() when mixerChannel != null:
return mixerChannel(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class TrackAutomationTargetDto_Generator extends TrackAutomationTargetDto {
  const TrackAutomationTargetDto_Generator({required this.paramId}): super._();
  

 final  int paramId;

/// Create a copy of TrackAutomationTargetDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$TrackAutomationTargetDto_GeneratorCopyWith<TrackAutomationTargetDto_Generator> get copyWith => _$TrackAutomationTargetDto_GeneratorCopyWithImpl<TrackAutomationTargetDto_Generator>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TrackAutomationTargetDto_Generator&&(identical(other.paramId, paramId) || other.paramId == paramId));
}


@override
int get hashCode => Object.hash(runtimeType,paramId);

@override
String toString() {
  return 'TrackAutomationTargetDto.generator(paramId: $paramId)';
}


}

/// @nodoc
abstract mixin class $TrackAutomationTargetDto_GeneratorCopyWith<$Res> implements $TrackAutomationTargetDtoCopyWith<$Res> {
  factory $TrackAutomationTargetDto_GeneratorCopyWith(TrackAutomationTargetDto_Generator value, $Res Function(TrackAutomationTargetDto_Generator) _then) = _$TrackAutomationTargetDto_GeneratorCopyWithImpl;
@useResult
$Res call({
 int paramId
});




}
/// @nodoc
class _$TrackAutomationTargetDto_GeneratorCopyWithImpl<$Res>
    implements $TrackAutomationTargetDto_GeneratorCopyWith<$Res> {
  _$TrackAutomationTargetDto_GeneratorCopyWithImpl(this._self, this._then);

  final TrackAutomationTargetDto_Generator _self;
  final $Res Function(TrackAutomationTargetDto_Generator) _then;

/// Create a copy of TrackAutomationTargetDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? paramId = null,}) {
  return _then(TrackAutomationTargetDto_Generator(
paramId: null == paramId ? _self.paramId : paramId // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class TrackAutomationTargetDto_MixerChannel extends TrackAutomationTargetDto {
  const TrackAutomationTargetDto_MixerChannel(this.field0): super._();
  

 final  MixerChannelParamTargetDto field0;

/// Create a copy of TrackAutomationTargetDto
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$TrackAutomationTargetDto_MixerChannelCopyWith<TrackAutomationTargetDto_MixerChannel> get copyWith => _$TrackAutomationTargetDto_MixerChannelCopyWithImpl<TrackAutomationTargetDto_MixerChannel>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TrackAutomationTargetDto_MixerChannel&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'TrackAutomationTargetDto.mixerChannel(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $TrackAutomationTargetDto_MixerChannelCopyWith<$Res> implements $TrackAutomationTargetDtoCopyWith<$Res> {
  factory $TrackAutomationTargetDto_MixerChannelCopyWith(TrackAutomationTargetDto_MixerChannel value, $Res Function(TrackAutomationTargetDto_MixerChannel) _then) = _$TrackAutomationTargetDto_MixerChannelCopyWithImpl;
@useResult
$Res call({
 MixerChannelParamTargetDto field0
});


$MixerChannelParamTargetDtoCopyWith<$Res> get field0;

}
/// @nodoc
class _$TrackAutomationTargetDto_MixerChannelCopyWithImpl<$Res>
    implements $TrackAutomationTargetDto_MixerChannelCopyWith<$Res> {
  _$TrackAutomationTargetDto_MixerChannelCopyWithImpl(this._self, this._then);

  final TrackAutomationTargetDto_MixerChannel _self;
  final $Res Function(TrackAutomationTargetDto_MixerChannel) _then;

/// Create a copy of TrackAutomationTargetDto
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(TrackAutomationTargetDto_MixerChannel(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as MixerChannelParamTargetDto,
  ));
}

/// Create a copy of TrackAutomationTargetDto
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$MixerChannelParamTargetDtoCopyWith<$Res> get field0 {
  
  return $MixerChannelParamTargetDtoCopyWith<$Res>(_self.field0, (value) {
    return _then(_self.copyWith(field0: value));
  });
}
}

// dart format on
