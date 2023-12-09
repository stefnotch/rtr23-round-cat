#version 460

#include "common.glsl"

layout(location = 0) rayPayloadInEXT float shadowed;

void main()
{
    shadowed = 1.0;
}