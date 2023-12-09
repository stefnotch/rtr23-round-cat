#version 460
#extension GL_EXT_nonuniform_qualifier : enable
#extension GL_EXT_debug_printf : enable

#include "common.glsl"

layout(location = 0) rayPayloadInEXT float shadowed;

void main()
{
  shadowed = 1.0;
}