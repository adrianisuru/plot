attribute vec3 position;
uniform mat4 pm;
uniform mat4 wm;
void main() {
    gl_Position = pm * wm * vec4(position, 1.0);
}
