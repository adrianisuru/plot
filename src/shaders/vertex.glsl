attribute vec3 position;
attribute vec3 normal;
uniform mat4 pm;
uniform mat4 wm;
uniform mat4 nm;
varying highp vec3 lighting;
void main() {
    gl_Position = pm * wm * vec4(position, 1.0);
  
    //Lighting
    highp vec3 ambientLight = vec3(0.3, 0.3, 0.3);
    highp vec3 directionalLightColor = vec3(1, 1, 1);
    highp vec3 directionalVector = normalize(vec3(0.85, 0.8, 0.75));
    highp vec4 transformedNormal = nm * vec4(normal, 1.0);
    highp float directional = max(dot(transformedNormal.xyz, directionalVector), 0.0);
    lighting = ambientLight + (directionalLightColor * directional);
}


