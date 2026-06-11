# Original prompt (verbatim, 2026-06-11)

> Preserved for traceability. The refined version lives in `docs/VISION.md`;
> requirement decomposition in `docs/REQUIREMENTS.md`.

I’m going to create an application that combines Adobe Photoshop and Illustrator. It will
support PSD and AI files from both Photoshop and Illustrator, with a focus on flexible layer
management, group management, and smart object management. It must be compatible with numerous
color profiles and printing standards while also offering high compatibility with other
graphic design tools. It will be an application where users choose whether to focus on vector
image editing in Illustrator on a per-project basis, or whether to focus on raster image
editing in Photoshop. However, just like the older versions of Photoshop and Illustrator,
there must be a certain degree of compatibility between vector and raster editing functions.
It should also include image selection and editing tools that utilize AI, similar to the
latest versions of Photoshop and Illustrator, though generative AI features are not required.
It would be great if AI could run basic open-source AI CV (Computer Vision or Vision Model)
baseline models locally or allow users to enter the API address and key for cloud models in
the settings. In addition, it would be great to have all the core features available in the
latest versions of Photoshop and Illustrator. Like in older versions of Photoshop, it would
be nice to include features for editing image sources of 3D assets (such as generating normal
maps or bump maps). First, don’t generate code immediately. Instead, refine the previous
prompt as much as possible, document the detailed goals and prompts as context in the local
directory, and independently structure the local harness (.CLAUDE.md, skills, agents, specs,
etc.) to achieve those goals, review whether there are any issues with the goals and plan,
and build the application by implementing the plan sequentially from top-down, verifying and
modifying it on a per-feature basis. Given the sheer number of features and the heavy
graphical workload involved, it would be ideal if the program supported GPU hardware
acceleration. It should also allow for detailed configuration (program settings, control key
settings, shortcut key settings, etc.) and run on Windows (AMD64/x86/ARM) or Mac (AMD64,
Apple Silicon), Linux, and support technologies like CUDA and MPS (Metal Performance
Shaders). If this is not feasible, the goal should be to support Windows (AMD64) and CUDA.
If you anticipate that the workload will exceed your capacity, please save the context window
as a document in a local directory so that work can be resumed at any time, and then wrap up
your work.
