lr_renderer is a work-in-progress wgpu renderer for line rider tracks.

most of the current api is future internal details, there will be a "Layer Stack" object which glues all the rendering primitives together into what you need to render a full track, so be aware that any use of the api that currently exists will certainly break.

the test/example code in entityrig_test_app is released into the public domain, and the renderer itself is licensed under the LGPL.
