// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'mixer.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$UiMixerChannelParams {

 Object get field0;



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiMixerChannelParams&&const DeepCollectionEquality().equals(other.field0, field0));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(field0));

@override
String toString() {
  return 'UiMixerChannelParams(field0: $field0)';
}


}

/// @nodoc
class $UiMixerChannelParamsCopyWith<$Res>  {
$UiMixerChannelParamsCopyWith(UiMixerChannelParams _, $Res Function(UiMixerChannelParams) __);
}


/// Adds pattern-matching-related methods to [UiMixerChannelParams].
extension UiMixerChannelParamsPatterns on UiMixerChannelParams {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( UiMixerChannelParams_Volume value)?  volume,TResult Function( UiMixerChannelParams_Pan value)?  pan,TResult Function( UiMixerChannelParams_Mute value)?  mute,TResult Function( UiMixerChannelParams_InvertedPhase value)?  invertedPhase,TResult Function( UiMixerChannelParams_Solo value)?  solo,required TResult orElse(),}){
final _that = this;
switch (_that) {
case UiMixerChannelParams_Volume() when volume != null:
return volume(_that);case UiMixerChannelParams_Pan() when pan != null:
return pan(_that);case UiMixerChannelParams_Mute() when mute != null:
return mute(_that);case UiMixerChannelParams_InvertedPhase() when invertedPhase != null:
return invertedPhase(_that);case UiMixerChannelParams_Solo() when solo != null:
return solo(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( UiMixerChannelParams_Volume value)  volume,required TResult Function( UiMixerChannelParams_Pan value)  pan,required TResult Function( UiMixerChannelParams_Mute value)  mute,required TResult Function( UiMixerChannelParams_InvertedPhase value)  invertedPhase,required TResult Function( UiMixerChannelParams_Solo value)  solo,}){
final _that = this;
switch (_that) {
case UiMixerChannelParams_Volume():
return volume(_that);case UiMixerChannelParams_Pan():
return pan(_that);case UiMixerChannelParams_Mute():
return mute(_that);case UiMixerChannelParams_InvertedPhase():
return invertedPhase(_that);case UiMixerChannelParams_Solo():
return solo(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( UiMixerChannelParams_Volume value)?  volume,TResult? Function( UiMixerChannelParams_Pan value)?  pan,TResult? Function( UiMixerChannelParams_Mute value)?  mute,TResult? Function( UiMixerChannelParams_InvertedPhase value)?  invertedPhase,TResult? Function( UiMixerChannelParams_Solo value)?  solo,}){
final _that = this;
switch (_that) {
case UiMixerChannelParams_Volume() when volume != null:
return volume(_that);case UiMixerChannelParams_Pan() when pan != null:
return pan(_that);case UiMixerChannelParams_Mute() when mute != null:
return mute(_that);case UiMixerChannelParams_InvertedPhase() when invertedPhase != null:
return invertedPhase(_that);case UiMixerChannelParams_Solo() when solo != null:
return solo(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( double field0)?  volume,TResult Function( double field0)?  pan,TResult Function( bool field0)?  mute,TResult Function( bool field0)?  invertedPhase,TResult Function( bool field0)?  solo,required TResult orElse(),}) {final _that = this;
switch (_that) {
case UiMixerChannelParams_Volume() when volume != null:
return volume(_that.field0);case UiMixerChannelParams_Pan() when pan != null:
return pan(_that.field0);case UiMixerChannelParams_Mute() when mute != null:
return mute(_that.field0);case UiMixerChannelParams_InvertedPhase() when invertedPhase != null:
return invertedPhase(_that.field0);case UiMixerChannelParams_Solo() when solo != null:
return solo(_that.field0);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( double field0)  volume,required TResult Function( double field0)  pan,required TResult Function( bool field0)  mute,required TResult Function( bool field0)  invertedPhase,required TResult Function( bool field0)  solo,}) {final _that = this;
switch (_that) {
case UiMixerChannelParams_Volume():
return volume(_that.field0);case UiMixerChannelParams_Pan():
return pan(_that.field0);case UiMixerChannelParams_Mute():
return mute(_that.field0);case UiMixerChannelParams_InvertedPhase():
return invertedPhase(_that.field0);case UiMixerChannelParams_Solo():
return solo(_that.field0);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( double field0)?  volume,TResult? Function( double field0)?  pan,TResult? Function( bool field0)?  mute,TResult? Function( bool field0)?  invertedPhase,TResult? Function( bool field0)?  solo,}) {final _that = this;
switch (_that) {
case UiMixerChannelParams_Volume() when volume != null:
return volume(_that.field0);case UiMixerChannelParams_Pan() when pan != null:
return pan(_that.field0);case UiMixerChannelParams_Mute() when mute != null:
return mute(_that.field0);case UiMixerChannelParams_InvertedPhase() when invertedPhase != null:
return invertedPhase(_that.field0);case UiMixerChannelParams_Solo() when solo != null:
return solo(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class UiMixerChannelParams_Volume extends UiMixerChannelParams {
  const UiMixerChannelParams_Volume(this.field0): super._();
  

@override final  double field0;

/// Create a copy of UiMixerChannelParams
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiMixerChannelParams_VolumeCopyWith<UiMixerChannelParams_Volume> get copyWith => _$UiMixerChannelParams_VolumeCopyWithImpl<UiMixerChannelParams_Volume>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiMixerChannelParams_Volume&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'UiMixerChannelParams.volume(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $UiMixerChannelParams_VolumeCopyWith<$Res> implements $UiMixerChannelParamsCopyWith<$Res> {
  factory $UiMixerChannelParams_VolumeCopyWith(UiMixerChannelParams_Volume value, $Res Function(UiMixerChannelParams_Volume) _then) = _$UiMixerChannelParams_VolumeCopyWithImpl;
@useResult
$Res call({
 double field0
});




}
/// @nodoc
class _$UiMixerChannelParams_VolumeCopyWithImpl<$Res>
    implements $UiMixerChannelParams_VolumeCopyWith<$Res> {
  _$UiMixerChannelParams_VolumeCopyWithImpl(this._self, this._then);

  final UiMixerChannelParams_Volume _self;
  final $Res Function(UiMixerChannelParams_Volume) _then;

/// Create a copy of UiMixerChannelParams
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(UiMixerChannelParams_Volume(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as double,
  ));
}


}

/// @nodoc


class UiMixerChannelParams_Pan extends UiMixerChannelParams {
  const UiMixerChannelParams_Pan(this.field0): super._();
  

@override final  double field0;

/// Create a copy of UiMixerChannelParams
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiMixerChannelParams_PanCopyWith<UiMixerChannelParams_Pan> get copyWith => _$UiMixerChannelParams_PanCopyWithImpl<UiMixerChannelParams_Pan>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiMixerChannelParams_Pan&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'UiMixerChannelParams.pan(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $UiMixerChannelParams_PanCopyWith<$Res> implements $UiMixerChannelParamsCopyWith<$Res> {
  factory $UiMixerChannelParams_PanCopyWith(UiMixerChannelParams_Pan value, $Res Function(UiMixerChannelParams_Pan) _then) = _$UiMixerChannelParams_PanCopyWithImpl;
@useResult
$Res call({
 double field0
});




}
/// @nodoc
class _$UiMixerChannelParams_PanCopyWithImpl<$Res>
    implements $UiMixerChannelParams_PanCopyWith<$Res> {
  _$UiMixerChannelParams_PanCopyWithImpl(this._self, this._then);

  final UiMixerChannelParams_Pan _self;
  final $Res Function(UiMixerChannelParams_Pan) _then;

/// Create a copy of UiMixerChannelParams
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(UiMixerChannelParams_Pan(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as double,
  ));
}


}

/// @nodoc


class UiMixerChannelParams_Mute extends UiMixerChannelParams {
  const UiMixerChannelParams_Mute(this.field0): super._();
  

@override final  bool field0;

/// Create a copy of UiMixerChannelParams
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiMixerChannelParams_MuteCopyWith<UiMixerChannelParams_Mute> get copyWith => _$UiMixerChannelParams_MuteCopyWithImpl<UiMixerChannelParams_Mute>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiMixerChannelParams_Mute&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'UiMixerChannelParams.mute(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $UiMixerChannelParams_MuteCopyWith<$Res> implements $UiMixerChannelParamsCopyWith<$Res> {
  factory $UiMixerChannelParams_MuteCopyWith(UiMixerChannelParams_Mute value, $Res Function(UiMixerChannelParams_Mute) _then) = _$UiMixerChannelParams_MuteCopyWithImpl;
@useResult
$Res call({
 bool field0
});




}
/// @nodoc
class _$UiMixerChannelParams_MuteCopyWithImpl<$Res>
    implements $UiMixerChannelParams_MuteCopyWith<$Res> {
  _$UiMixerChannelParams_MuteCopyWithImpl(this._self, this._then);

  final UiMixerChannelParams_Mute _self;
  final $Res Function(UiMixerChannelParams_Mute) _then;

/// Create a copy of UiMixerChannelParams
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(UiMixerChannelParams_Mute(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}


}

/// @nodoc


class UiMixerChannelParams_InvertedPhase extends UiMixerChannelParams {
  const UiMixerChannelParams_InvertedPhase(this.field0): super._();
  

@override final  bool field0;

/// Create a copy of UiMixerChannelParams
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiMixerChannelParams_InvertedPhaseCopyWith<UiMixerChannelParams_InvertedPhase> get copyWith => _$UiMixerChannelParams_InvertedPhaseCopyWithImpl<UiMixerChannelParams_InvertedPhase>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiMixerChannelParams_InvertedPhase&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'UiMixerChannelParams.invertedPhase(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $UiMixerChannelParams_InvertedPhaseCopyWith<$Res> implements $UiMixerChannelParamsCopyWith<$Res> {
  factory $UiMixerChannelParams_InvertedPhaseCopyWith(UiMixerChannelParams_InvertedPhase value, $Res Function(UiMixerChannelParams_InvertedPhase) _then) = _$UiMixerChannelParams_InvertedPhaseCopyWithImpl;
@useResult
$Res call({
 bool field0
});




}
/// @nodoc
class _$UiMixerChannelParams_InvertedPhaseCopyWithImpl<$Res>
    implements $UiMixerChannelParams_InvertedPhaseCopyWith<$Res> {
  _$UiMixerChannelParams_InvertedPhaseCopyWithImpl(this._self, this._then);

  final UiMixerChannelParams_InvertedPhase _self;
  final $Res Function(UiMixerChannelParams_InvertedPhase) _then;

/// Create a copy of UiMixerChannelParams
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(UiMixerChannelParams_InvertedPhase(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}


}

/// @nodoc


class UiMixerChannelParams_Solo extends UiMixerChannelParams {
  const UiMixerChannelParams_Solo(this.field0): super._();
  

@override final  bool field0;

/// Create a copy of UiMixerChannelParams
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiMixerChannelParams_SoloCopyWith<UiMixerChannelParams_Solo> get copyWith => _$UiMixerChannelParams_SoloCopyWithImpl<UiMixerChannelParams_Solo>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiMixerChannelParams_Solo&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'UiMixerChannelParams.solo(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $UiMixerChannelParams_SoloCopyWith<$Res> implements $UiMixerChannelParamsCopyWith<$Res> {
  factory $UiMixerChannelParams_SoloCopyWith(UiMixerChannelParams_Solo value, $Res Function(UiMixerChannelParams_Solo) _then) = _$UiMixerChannelParams_SoloCopyWithImpl;
@useResult
$Res call({
 bool field0
});




}
/// @nodoc
class _$UiMixerChannelParams_SoloCopyWithImpl<$Res>
    implements $UiMixerChannelParams_SoloCopyWith<$Res> {
  _$UiMixerChannelParams_SoloCopyWithImpl(this._self, this._then);

  final UiMixerChannelParams_Solo _self;
  final $Res Function(UiMixerChannelParams_Solo) _then;

/// Create a copy of UiMixerChannelParams
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(UiMixerChannelParams_Solo(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}


}

// dart format on
