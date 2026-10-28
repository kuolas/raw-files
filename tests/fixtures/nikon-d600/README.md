# The D600 session

The 56 NEF files of the session on which I measured the autofocus points
of my own Nikon D600, on 2026-10-26: one frame per point in single-point
autofocus, then the control frames, in the order they were shot; and
the camera's JPEG of frame 4. The NEF notes record the session, what
each frame shows and every reading made from it
([autofocus-points.md](../../../docs/formats/nef/autofocus-points.md#2-nikon-d600)),
and number the frames 1 to 56 as this page does. Each NEF file is the
camera's NEF with its pixels and personal fields removed and its
structure and values kept, 2,452 bytes; the JPEG is the camera's JPEG
with its images replaced by stubs and its personal fields zeroed, its
Exif segment otherwise as the camera wrote it, 69,876 bytes;
[the fixtures' README](../README.md#the-d600-session) says how they were
stripped and checked, under which licence they are published, and who
*I* is.

Frames 4, 42 and 49, in bold, are the three that the record chooses to
check the table of the D600's points with: a single point far from the
centre, where a wrong table, a wrong bit order or a wrong frame shows,
as a NEF and as its JPEG;
an auto-area frame, whose mask marks 12 points used, the primary among
them; and a single point in the DX crop, which shifts into the crop.

| Frame | File | Camera's file name | Shot | SHA-256 |
|---|---|---|---|---|
| 1 | `frame-01.nef` | `AK6_7567.NEF` | single point 13 | `8a1af83e9a6fdb1ad5ee50f79400b85c244997dff600cb41881d528f765b8132` |
| 2 | `frame-02.nef` | `AK6_7568.NEF` | single point 3 | `d0230841586c0ce773ec80ccdb0844adc63fac085fd5fb372c2e9518a54b36b1` |
| 3 | `frame-03.nef` | `AK6_7569.NEF` | single point 8 | `41f1f8d4694d24f1d3d8c4f1d60366057c2f7fc98fdc9ca23d022c348c7e0448` |
| 4 | `frame-04.nef` | `AK6_7570.NEF` | **single point 38** (the table's tests) | `6cf5c10f3db3f072a92853bf4f89067c8adc656bced6723b9ceefaa58c83558b` |
| 4 | `frame-04.jpg` | `AK6_7570.JPG` | **single point 38**, the camera's JPEG of the same shot (the table's tests) | `59ccd4c4c8248cf97824a64c118f1b84439e1104ac1e94c841872bccdcbb032c` |
| 5 | `frame-05.nef` | `AK6_7571.NEF` | single point 35 | `78d31f7eb3377d4a7a6cc3ce993a4866195757f2fd621c2785b8868194ff9545` |
| 6 | `frame-06.nef` | `AK6_7572.NEF` | single point 32 | `ebb5b028f6181a023a2dd53494ff6b87845e3f6d812c5c3203f32353c6323857` |
| 7 | `frame-07.nef` | `AK6_7573.NEF` | single point 29 | `8e7e3daf57559b3b957e273392f2de461781b12ebc98ab6bbf7c759c627d9be1` |
| 8 | `frame-08.nef` | `AK6_7574.NEF` | single point 12 | `0711762d4b2181744152aea60fd465b112002947db0d03de29c5bae49db4d264` |
| 9 | `frame-09.nef` | `AK6_7575.NEF` | single point 2 | `3007b264ce9ea44eefb6f92dfcde65e68f8ebdc6bc659da96e54d55c112b6133` |
| 10 | `frame-10.nef` | `AK6_7576.NEF` | single point 7 | `be2b9ddb1d711c67a984652a02c2f196078c38db39fc0f2d40e2da42c01bad44` |
| 11 | `frame-11.nef` | `AK6_7577.NEF` | single point 17 | `161eaf0718b8a480d20a4ecd7b74413c1eb81c6b56f7abedb1f4ac6320ffd5c5` |
| 12 | `frame-12.nef` | `AK6_7578.NEF` | single point 20 | `e5c8ca0bbd2e68ab2d58fd5bea7189d9d684ca283da0e2a76e59d49da202921e` |
| 13 | `frame-13.nef` | `AK6_7579.NEF` | single point 23 | `445b7b41622c40afada7e20a15d49236a55467b4276fc2c46ae754c66735cba6` |
| 14 | `frame-14.nef` | `AK6_7580.NEF` | single point 26 | `d2bc7e0927aba7b5b75f73341404213092ff4e6c19f94c336b1232d91c640850` |
| 15 | `frame-15.nef` | `AK6_7581.NEF` | single point 37 | `4d146851bb30d27d470a15c37a2d51200686d5cb1b313d0b185688ee788b8bec` |
| 16 | `frame-16.nef` | `AK6_7582.NEF` | single point 34 | `ca5594a3149cda50279d6218a1331c7d539a409d8cd5827a767aeb852529de8b` |
| 17 | `frame-17.nef` | `AK6_7583.NEF` | single point 31 | `00d8c26e9dd33a2b165eb467ec463622e120347e3efb6fcaac32a52da94f0065` |
| 18 | `frame-18.nef` | `AK6_7584.NEF` | single point 28 | `55eaf4751f719fbfe03587fb27a9524408cb35f15caaf661abc266c0ee5725a7` |
| 19 | `frame-19.nef` | `AK6_7585.NEF` | single point 11 | `832da277c7412710b22d336d0f682a043dad48cd195e9898e7f225a48ae47477` |
| 20 | `frame-20.nef` | `AK6_7586.NEF` | single point 1 | `3eb8da527a5a14566bfb14fa67e402e00758b41aae8badbd41ea09c2baf1034a` |
| 21 | `frame-21.nef` | `AK6_7587.NEF` | single point 6 | `bd74050b53d97793b2c015d11c36b68e06907329f1ac58a05250a520f44d6b58` |
| 22 | `frame-22.nef` | `AK6_7588.NEF` | single point 16 | `20edfcb176ec2530e493f27c83b6a1a02b479c3884b7218855ad6623785ea7e7` |
| 23 | `frame-23.nef` | `AK6_7589.NEF` | single point 19 | `d12fc584c80bc1db3a5601fd5512d4cdca92c7037d6b19b3b8642f60d07fdfa3` |
| 24 | `frame-24.nef` | `AK6_7590.NEF` | single point 22 | `d798bcdc4da49335e0f0b6d51c0be93b4370df3dc414e2927c288cb42e598779` |
| 25 | `frame-25.nef` | `AK6_7591.NEF` | single point 25 | `cb3e44dfa74a951f78827f8017de5ba5f7cea2ea2556a2916218ddc413641fa7` |
| 26 | `frame-26.nef` | `AK6_7592.NEF` | single point 39 | `d1b3647c284c14ec55e9f137969ca7c7cc02ab2b758829a48b3052acd3aa7de9` |
| 27 | `frame-27.nef` | `AK6_7593.NEF` | single point 36 | `03ef5f8f2595f725eae0eebb4ab610f6ebf2e6ec0dde54400370b72ec47d3cd2` |
| 28 | `frame-28.nef` | `AK6_7594.NEF` | single point 33 | `f500a1c07cfb09d0f9e5b1f01eca07e87704121cce80ecf8fe4cef5d03537840` |
| 29 | `frame-29.nef` | `AK6_7595.NEF` | single point 30 | `b7f9483dd265ea125b855eb101478f330f2f55541b7998b6730a19afe6134667` |
| 30 | `frame-30.nef` | `AK6_7596.NEF` | single point 14 | `c8bb615866aa9001ec94bece1111a65018a36287f4566cd8b80212387731c75b` |
| 31 | `frame-31.nef` | `AK6_7597.NEF` | single point 4 | `0d6646a6e1c932b45b9d78c69aa3e3760ce2e0b6a9ee3a701384cf5b7ecbf0f6` |
| 32 | `frame-32.nef` | `AK6_7598.NEF` | single point 9 | `0ef4b5974fac4f8a1fe85e7dbca8b1204c9a59380d0da5857259972855738411` |
| 33 | `frame-33.nef` | `AK6_7599.NEF` | single point 18 | `a307ff4932c963b8590e58a9587f403d339620e332ee1253a69b1844f188c759` |
| 34 | `frame-34.nef` | `AK6_7600.NEF` | single point 21 | `2da6fcf1d5ba5233c464ffb08b1b8f4f71320d21209d7bfa4e8896d9fb63fe75` |
| 35 | `frame-35.nef` | `AK6_7601.NEF` | single point 24 | `af320617ef479b2e45745bd244124b0b6079bff112b8d000a5baf9f608572d3a` |
| 36 | `frame-36.nef` | `AK6_7602.NEF` | single point 27 | `cf33ae8d7baa577245e7425166782599800bb1561f79296a70b6e98090064667` |
| 37 | `frame-37.nef` | `AK6_7603.NEF` | single point 15 | `e28d6c8ba02e63b1682b4024340062417abb15509098f6031c9f284e88cd0caa` |
| 38 | `frame-38.nef` | `AK6_7604.NEF` | single point 5 | `caae6f2d8fb188db7d9b52ae3f4caf145cda23432ab96b5d0a8fb28d2c325865` |
| 39 | `frame-39.nef` | `AK6_7605.NEF` | single point 10 | `54009f02289fe3841cec6716fae490d0393561bf7881812f3d1da3f025d67e43` |
| 40 | `frame-40.nef` | `AK6_7606.NEF` | single point 1, a repeat | `213b4dc5ad7a5aca9872a550eccba9529099536c253f7ac9abd6b3494b936c27` |
| 41 | `frame-41.nef` | `AK6_7607.NEF` | single point 13, a repeat | `2c8a1385ec874803ab190556fd14d78b813f960d0d4519dac3c61eab1da9223f` |
| 42 | `frame-42.nef` | `AK6_7608.NEF` | **auto-area, 12 points used, primary point 22** (the table's tests) | `ffb4e24e81659344f9f2ca0b097ef3569c63bfac143936498f34a3762fc5b7af` |
| 43 | `frame-43.nef` | `AK6_7609.NEF` | auto-area, 13 points used, primary point 22 | `c807db22ae2e2a9a9731ef30dd3f107f43d66671117c934782bc3405d672671e` |
| 44 | `frame-44.nef` | `AK6_7610.NEF` | auto-area, 9 points used, primary point 4 | `ed36c08e99ddc97260a0aad15985c9e696df05b0dc2ab29dfed8761fd893a2b4` |
| 45 | `frame-45.nef` | `AK6_7611.NEF` | manual focus | `e4da3d8be1b4a80dbeb4079e87b2dec38fb7013637d4c8ba1b9c31164ec51299` |
| 46 | `frame-46.nef` | `AK6_7612.NEF` | manual focus | `309e467bd0301761028c4b2239634d5fa08347aa8ec638562805dfb83d3229b2` |
| 47 | `frame-47.nef` | `AK6_7613.NEF` | single point 13, portrait | `1075ade51377cafe416a01142ba324b3237c92ab6f8477900435284a863f7e50` |
| 48 | `frame-48.nef` | `AK6_7614.NEF` | single point 38, portrait | `5e83fd53315a1d7da8154ef2c6a741e3cbc30c180650a1d10f00c6f19feac854` |
| 49 | `frame-49.nef` | `AK6_7615.NEF` | **single point 38, DX** (the table's tests) | `5c7622565e247c86a3e0af8b153fddb0d0fd034427632c52e57423d245f2c999` |
| 50 | `frame-50.nef` | `AK6_7616.NEF` | single point 27, DX | `eb3f5c68ccbb57b6620438ee321200a9cca3fb6916a8d8bf8368c14d878eb913` |
| 51 | `frame-51.nef` | `AK6_7617.NEF` | dynamic area, 21 points, point 1, DX | `2afd5a351042195ffc502a642b02724d34352aa68c3a5ae931fea5374a3459b1` |
| 52 | `frame-52.nef` | `AK6_7618.NEF` | dynamic area, 21 points, point 1 | `85e0d39dbe7a6dac0468c03c0075c23d40c449c04693af19ade344d8155ef277` |
| 53 | `frame-53.nef` | `AK6_7619.NEF` | live view, the area at the bottom left | `9c03ff9974e9f5335c6c31cf374515167461f14752645dc55ffa6be0c133052b` |
| 54 | `frame-54.nef` | `AK6_7620.NEF` | live view, the area at the top left | `81247501564f1b6ef08ddcf89b78c45dd7001fa664b046fa95ae99681b36b256` |
| 55 | `frame-55.nef` | `AK6_7621.NEF` | live view, the area at the top right | `1507624576d3746ab4db4d131dfede99237659b8f96ac134243ad222c3fa24d7` |
| 56 | `frame-56.nef` | `AK6_7622.NEF` | live view, the area at the bottom right | `433183b3d80363167d6b51fff76531161f79e256b8bd8f0fd057c8f14018b11c` |
