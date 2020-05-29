varying highp vec3 lighting;
void main() {
    highp vec4 color = vec4(0.5, 0.5, 0.5, 1.0);
    gl_FragColor = vec4(color.rgb * lighting, color.a);
}
