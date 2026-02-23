/* Cellular Automata on the torn paper background.
 *
 * Modes cycle each page load via cookie:
 *   edge_growth → glider_chaos → random_seed → repeat
 *
 * To add a new mode, define an object with { init, tick } methods and
 * append its key to MODE_ORDER. The mode can use any of the shared
 * helpers (grid, setCell, getCell, stepGameOfLife, placePattern, etc.).
 */
(function() {
    'use strict';

    // ================================================================
    // Configuration
    // ================================================================

    var CELL_PX   = 4;    // CSS pixels per cell
    var TPS       = 8;   // simulation ticks per second
    var CELL_RGBA = [0, 0, 30, 11]; // [R, G, B, A] — faint blue-gray

    // Order in which modes are shown (cycles via cookie)
    var MODE_ORDER = ['edge_growth', 'glider_chaos', 'random_seed'];

    // ================================================================
    // Canvas & grid setup
    // ================================================================

    var canvas = document.getElementById('ca-canvas');
    if (!canvas) return;
    var ctx = canvas.getContext('2d');

    // Size the canvas to cover the full paper-tear region.
    // Fixed pixel size so it doesn't squash on window resize.
    var canvasW = Math.max(screen.width, document.documentElement.clientWidth, 2000);
    var canvasH = 1200;
    canvas.style.width  = canvasW + 'px';
    canvas.style.height = canvasH + 'px';

    var cols = Math.ceil(canvasW / CELL_PX);
    var rows = Math.ceil(canvasH / CELL_PX);
    canvas.width  = cols;
    canvas.height = rows;

    var grid    = new Uint8Array(cols * rows);
    var gridBuf = new Uint8Array(cols * rows);
    var imgData = ctx.createImageData(cols, rows);

    // ================================================================
    // Shared helpers — available to all modes
    // ================================================================

    /** Read a cell (0 for out-of-bounds). */
    function getCell(x, y) {
        return (x >= 0 && x < cols && y >= 0 && y < rows)
            ? grid[y * cols + x] : 0;
    }

    /** Set a cell (bounds-checked). */
    function setCell(x, y, v) {
        if (x >= 0 && x < cols && y >= 0 && y < rows)
            grid[y * cols + x] = v;
    }

    /** Seed random cells across the entire grid. */
    function seedRandom(density) {
        for (var i = 0; i < grid.length; i++)
            if (Math.random() < density) grid[i] = 1;
    }

    /** Sprinkle n random live cells anywhere on the grid. */
    function sprinkle(n) {
        for (var i = 0; i < n; i++)
            grid[Math.floor(Math.random() * grid.length)] = 1;
    }

    /**
     * Place a pattern on the grid at (cx, cy).
     * Pattern is an array of [row, col] offsets.
     */
    function placePattern(pattern, cx, cy) {
        for (var i = 0; i < pattern.length; i++)
            setCell(cx + pattern[i][1], cy + pattern[i][0], 1);
    }

    /** One generation of Conway's Game of Life (double-buffered). */
    function stepGameOfLife() {
        for (var y = 0; y < rows; y++) {
            for (var x = 0; x < cols; x++) {
                var n = getCell(x-1,y-1) + getCell(x,y-1) + getCell(x+1,y-1) +
                        getCell(x-1,y)                     + getCell(x+1,y) +
                        getCell(x-1,y+1) + getCell(x,y+1) + getCell(x+1,y+1);
                var i = y * cols + x;
                gridBuf[i] = grid[i] ? ((n === 2 || n === 3) ? 1 : 0)
                                     : ((n === 3) ? 1 : 0);
            }
        }
        var t = grid; grid = gridBuf; gridBuf = t;
    }

    /**
     * Compute the y-coordinate of the diagonal tear edge at column x
     * (in grid units). Useful for modes that seed along the paper boundary.
     */
    function tearEdgeY(x) {
        var leftY  = 900 / CELL_PX;
        var rightY = (900 - canvasW * Math.tan(8 * Math.PI / 180)) / CELL_PX;
        return Math.floor(leftY + (x / cols) * (rightY - leftY));
    }

    // ================================================================
    // Pattern library
    // ================================================================

    var PATTERNS = {
        // Standard glider in 4 directions
        glider_SE: [[0,1],[1,2],[2,0],[2,1],[2,2]],
        glider_SW: [[0,1],[1,0],[2,0],[2,1],[2,2]],
        glider_NE: [[0,0],[0,1],[0,2],[1,2],[2,1]],
        glider_NW: [[0,0],[0,1],[0,2],[1,0],[2,1]],

        // Lightweight spaceship (LWSS) in 4 directions
        lwss_E: [[0,1],[0,4],[1,0],[2,0],[2,4],[3,0],[3,1],[3,2],[3,3]],
        lwss_W: [[0,0],[0,3],[1,4],[2,0],[2,4],[3,1],[3,2],[3,3],[3,4]],
        lwss_S: [[0,3],[1,0],[1,3],[2,3],[3,3],[4,0],[4,1],[4,2],[4,3]],
        lwss_N: [[0,0],[0,1],[0,2],[0,3],[1,0],[2,0],[3,0],[3,3],[4,3]],

        // R-pentomino — chaotic long-lived methuselah
        r_pentomino: [[0,1],[0,2],[1,0],[1,1],[2,1]]
    };

    /** Place a randomly-chosen pattern from a list of pattern keys. */
    function placeRandomPattern(keys, cx, cy) {
        var key = keys[Math.floor(Math.random() * keys.length)];
        placePattern(PATTERNS[key], cx, cy);
    }

    // All spaceship/methuselah pattern keys (for glider_chaos mode)
    var ALL_SHIPS = [
        'glider_SE', 'glider_SW', 'glider_NE', 'glider_NW',
        'lwss_E', 'lwss_W', 'lwss_S', 'lwss_N',
        'r_pentomino'
    ];

    // ================================================================
    // Modes — each is { init(), tick() }
    // ================================================================

    var MODES = {};

    // --- Random seeding + Game of Life ---
    MODES.random_seed = {
        init: function() {
            seedRandom(0.25);
        },
        tick: function() {
            sprinkle(50);
            stepGameOfLife();
        }
    };

    // --- Growing from diagonal tear edge + Game of Life ---
    MODES.edge_growth = (function() {
        var spread;
        return {
            init: function() {
                spread = 4;
            },
            tick: function() {
                spread = Math.min(spread + 2.0, rows * 0.8);
                var s = Math.floor(spread);
                for (var x = 0; x < cols; x++) {
                    var edgeY = tearEdgeY(x);
                    for (var dy = -s; dy <= 0; dy++) {
                        var y = edgeY + dy;
                        if (y >= 0 && y < rows && Math.random() < 0.06)
                            grid[y * cols + x] = 1;
                    }
                }
                stepGameOfLife();
            }
        };
    })();

    // --- Grid packed with gliders, LWSS, and R-pentominoes ---
    MODES.glider_chaos = {
        init: function() {
            var spacing = 12;
            for (var gy = 2; gy < rows - 2; gy += spacing) {
                for (var gx = 2; gx < cols - 2; gx += spacing) {
                    var jx = gx + Math.floor(Math.random() * 5) - 2;
                    var jy = gy + Math.floor(Math.random() * 5) - 2;
                    placeRandomPattern(ALL_SHIPS, jx, jy);
                }
            }
        },
        tick: function() {
            stepGameOfLife();
        }
    };

    // ================================================================
    // Rendering
    // ================================================================

    function draw() {
        var d = imgData.data;
        d.fill(0);
        for (var i = 0; i < grid.length; i++) {
            if (grid[i]) {
                var p = i << 2;
                d[p]     = CELL_RGBA[0];
                d[p + 1] = CELL_RGBA[1];
                d[p + 2] = CELL_RGBA[2];
                d[p + 3] = CELL_RGBA[3];
            }
        }
        ctx.putImageData(imgData, 0, 0);
    }

    // ================================================================
    // Mode selection (cycles via cookie)
    // ================================================================

    var prev = document.cookie.replace(/(?:^|.*;\s*)ca_seq\s*=\s*(\d+).*$/, '$1');
    var seq  = /^\d+$/.test(prev) ? (parseInt(prev, 10) + 1) % MODE_ORDER.length : 0;
    document.cookie = 'ca_seq=' + seq + ';path=/;max-age=31536000;SameSite=Lax';

    var current = MODES[MODE_ORDER[seq]];
    current.init();

    // ================================================================
    // Main loop
    // ================================================================

    var tickMs   = 1000 / TPS;
    var lastTick = 0;

    function loop(ts) {
        if (ts - lastTick >= tickMs) {
            current.tick();
            draw();
            lastTick = ts;
        }
        requestAnimationFrame(loop);
    }
    requestAnimationFrame(loop);
})();
