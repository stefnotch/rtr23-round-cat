#version 460
#extension GL_EXT_nonuniform_qualifier : enable

#include "common.glsl"

layout(location = 0) rayPayloadInEXT float shadowed;

void main()
{
  shadowed = 1.0;
}