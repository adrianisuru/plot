   // Use ES module import syntax to import functionality from the module
      // that we have compiled.
      //
      // Note that the `default` import is an initialization function which
      // will "boot" the module and make it ready to use. Currently browsers
      // don't support natively imported WebAssembly as an ES module, but
      // eventually the manual initialization won't be required!
import ('./pkg').then(wasm=>{

        let app = new wasm.App();

        var vendors = ['webkit', 'moz'];
for (var x = 0; x < vendors.length && !window.requestAnimationFrame; ++x) {
    window.requestAnimationFrame = window[vendors[x] + 'RequestAnimationFrame'];
    window.cancelAnimationFrame = window[vendors[x] + 'CancelAnimationFrame'] || window[vendors[x] + 'CancelRequestAnimationFrame'];
}
        
        var lastTime = (new Date()).getTime(),
            currentTime = 0,
            delta = 0;

        function gameLoop() {
            window.requestAnimationFrame(gameLoop);
        
            currentTime = (new Date()).getTime();
            delta = (currentTime - lastTime) / 1000;

            app.update();

            app.render();
            
            lastTime = currentTime;
        }

        gameLoop();
});
